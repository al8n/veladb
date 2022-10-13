use std::collections::HashSet;

use crate::{error::*, IteratorOptions};
use bable::{
    kvstructs::{compare_key, KeyExt, KeyRange, KeyRef, Value, ValueRef},
    BableIterator, Flag, Table, TableIterator, ConcatTableIterator,
};
use indexsort::{search, sort_slice};
use parking_lot::RwLock;
use vela_utils::ref_counter::RefCounter;

pub(super) struct LevelHandler {
    inner: RefCounter<RwLock<Inner>>,

    // The following are initialized once and const.
    level: usize,
    max_levels: usize,
    num_level_zero_table_stall: usize,
}

struct Inner {
    tables: Vec<Table>,
    total_size: u64,
    total_stale_size: u64,
}

impl Default for Inner {
    fn default() -> Self {
        Self::new()
    }
}

impl Inner {
    #[inline]
    const fn new() -> Self {
        Self {
            tables: Vec::new(),
            total_size: 0,
            total_stale_size: 0,
        }
    }
}

impl LevelHandler {
    #[inline]
    pub(super) fn new(level: usize, max_levels: usize, num_level_zero_table_stall: usize) -> Self {
        Self {
            level,
            max_levels,
            inner: RefCounter::new(RwLock::new(Inner::new())),
            num_level_zero_table_stall,
        }
    }

    #[inline]
    pub(super) fn is_last_level(&self) -> bool {
        self.level == self.max_levels - 1
    }

    #[inline]
    pub(super) fn get_total_stale_size(&self) -> u64 {
        self.inner.read().total_stale_size
    }

    #[inline]
    pub(super) fn get_total_size(&self) -> u64 {
        self.inner.read().total_size
    }

    /// Remove tables idx0, ..., idx1-1.
    pub(super) fn remove_tables(&self, to_del: &[Table]) {
        let mut inner = self.inner.write();

        let to_del_map = to_del.iter().map(|t| t.id()).collect::<HashSet<_>>();

        let mut total_size = inner.total_size;
        let mut total_stale_size = inner.total_stale_size;
        inner.tables.retain(|t| {
            if to_del_map.contains(&t.id()) {
                total_size -= t.table_size() as u64;
                total_stale_size -= t.stale_data_size() as u64;
                false
            } else {
                true
            }
        });

        inner.total_size = total_size;
        inner.total_stale_size = total_stale_size;
    }

    /// Replace `tables[left..right]` with newTables. Note this EXCLUDES tables[right].
    pub(super) fn replace_tables(&self, to_del: &[Table], to_add: &[Table]) {
        // Need to re-search the range of tables in this level to be replaced as other goroutines might
        // be changing it as well.  (They can't touch our tables, but if they add/remove other tables,
        // the indices get shifted around.)
        let mut inner = self.inner.write();

        let to_del_map = to_del.iter().map(|t| t.id()).collect::<HashSet<_>>();

        let mut total_size = inner.total_size;
        let mut total_stale_size = inner.total_stale_size;
        let mut add_total_size = 0;
        let mut add_total_stale_size = 0;
        inner.tables.retain(|t| {
            if to_del_map.contains(&t.id()) {
                total_size -= t.table_size() as u64;
                total_stale_size -= t.stale_data_size() as u64;
                false
            } else {
                true
            }
        });
        inner.tables.extend(to_add.iter().map(|t| {
            add_total_size += t.table_size() as u64;
            add_total_stale_size += t.stale_data_size() as u64;
            t.clone()
        }));

        inner.total_size = total_size;
        inner.total_size += add_total_size;
        inner.total_stale_size = total_stale_size;
        inner.total_stale_size += add_total_stale_size;

        sort_slice(&mut inner.tables, |data, i, j| {
            let i_s = data[i].smallest();
            let j_s = data[j].smallest();
            i_s < j_s
        });
    }

    /// Adds toAdd table to [`LevelHandler`]. Normally when we add tables to levelHandler, we sort
    /// tables based on `table.smallest`. This is required for correctness of the system. But in case of
    /// stream writer this can be avoided. We can just add tables to levelHandler's table list
    /// and after all `add_table` calls, we can sort table list(check sortTable method).
    /// NOTE: `LevelHandler::sort_tables(&self)` should be called after call addTable calls are done.
    #[inline]
    pub(super) fn add_table(&self, t: &Table) {
        let mut inner = self.inner.write();
        inner.total_size += t.table_size() as u64;
        inner.total_stale_size += t.stale_data_size() as u64;
        inner.tables.push(t.clone());
    }

    /// Sorts tables in [`LevelHandler`] based on `table.smallest`.
    /// Normally it should be called after all addTable calls.
    #[inline]
    pub(super) fn sort_tables(&self) {
        let mut inner = self.inner.write();
        sort_slice(&mut inner.tables, |data, i, j| {
            let i_s = data[i].smallest();
            let j_s = data[j].smallest();
            i_s < j_s
        });
    }

    /// Returns true if ok and no stalling.
    pub(super) fn try_add_level0_table(&self, t: &Table) -> bool {
        assert_eq!(self.level, 0);

        // Need lock as we may be deleting the first table during a level 0 compaction.
        let mut inner = self.inner.write();

        // Stall (by returning false) if we are above the specified stall setting for L0.
        if inner.tables.len() >= self.num_level_zero_table_stall {
            return false;
        }

        inner.total_size = t.table_size() as u64;
        inner.total_stale_size = t.stale_data_size() as u64;
        inner.tables.push(t.clone());

        true
    }

    pub(super) fn get_table_for_key(&self, key: KeyRef) -> Vec<Table> {
        let inner = self.inner.read();
        if self.level == 0 {
            // For level 0, we need to check every table. Remember to make a copy as s.tables may change
            // once we exit this function, and we don't want to lock s.tables while seeking in tables.
            // CAUTION: Reverse the tables.
            let mut out = inner.tables.clone();
            out.reverse();
            return out;
        }

        // For level >= 1, we can do a binary search as key range does not overlap.
        let idx = search(inner.tables.len(), |i| {
            inner.tables[i].biggest().as_key_ref() >= key.as_key_ref()
        });

        if idx >= inner.tables.len() {
            // Given key is strictly > than every element we have.
            return Vec::new();
        }

        vec![inner.tables[idx].clone()]
    }

    /// Returns value for a given key or the key after that. If not found, return `None`.
    pub(super) fn get(&self, key: impl KeyExt) -> Result<Option<Value>> {
        let key = key.as_key_ref();
        let tables = self.get_table_for_key(key);

        let key_no_ts = key.parse_key();
        let hash = bable::bloom::hash(key_no_ts);
        let mut max_vs: Option<Value> = None;
        for table in tables {
            if table.contains_hash(hash) {
                // TODO: metrics
                continue;
            }

            let mut iter = table.iter(Flag::NONE);

            // TODO: metrics
            iter.seek(key);

            if !iter.valid() {
                continue;
            }

            if let Some(ikey) = iter.key() {
                if ikey.same_key(key) {
                    let version = ikey.parse_timestamp();
                    if let Some(max_val) = &mut max_vs {
                        let val = iter.val().unwrap().to_value();
                        if max_val.get_version() < version {
                            *max_val = val.set_version(version);
                        }
                    } else {
                        max_vs = Some(iter.val().unwrap().to_value());
                    }
                }
            }
        }

        Ok(max_vs)
    }

    /// Returns an array of iterators, for merging.
    pub(super) fn iterators(&self, opt: &IteratorOptions) -> Vec<TableIterator> {
        let inner = self.inner.read();
        let topt = if opt.reverse { Flag::REVERSED } else { Flag::NONE };
        if self.level == 0 {
            // Remember to add in reverse order!
		    // The newer table at the end of s.tables should be added first as it takes precedence.
		    // Level 0 tables are not in key sorted order, so we need to consider them one by one.
            return inner.tables.iter().filter_map(|t| {
                if opt.pick_table(t) {
                    Some(TableIterator::from(t.iter(topt)))
                } else {
                    None
                }
            }).collect();
        }
        
        let tables = opt.pick_tables(&inner.tables);
        if tables.is_empty() {
            return Vec::new();
        }
        vec![TableIterator::from(ConcatTableIterator::new(tables, topt))]
    }

    pub(super) fn get_tables(&self, opt: &IteratorOptions) -> Vec<Table> {
        if opt.reverse {
            panic!("wisc: invalid option for get_tables");
        }

        // Typically this would only be called for the last level.
        let inner = self.inner.read();
        if self.level == 0 {
            return inner.tables.iter().filter(|t| opt.pick_table(t)).cloned().collect();
        }

        opt.pick_tables(&inner.tables)
    }

    pub(super) fn overlapping_tables<L: KeyExt, R: KeyExt>(
        &self,
        kr: KeyRange<L, R>,
    ) -> (usize, usize) {
        let left = kr.start();
        let right = kr.end();
        if left.as_bytes().is_empty() || right.as_bytes().is_empty() {
            return (0, 0);
        }

        let inner = self.inner.read();
        let left_idx = search(inner.tables.len(), |i| {
            match compare_key(left.as_key_ref(), inner.tables[i].biggest()) {
                core::cmp::Ordering::Less | core::cmp::Ordering::Equal => true,
                core::cmp::Ordering::Greater => false,
            }
        });

        let right_idx = search(inner.tables.len(), |i| {
            match compare_key(right.as_key_ref(), inner.tables[i].smallest()) {
                core::cmp::Ordering::Less => true,
                core::cmp::Ordering::Equal | core::cmp::Ordering::Greater => false,
            }
        });

        (left_idx, right_idx)
    }
}
