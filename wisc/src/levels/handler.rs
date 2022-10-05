use std::collections::HashSet;

use bable::{Table, kvstructs::{KeyExt, Value}};
use crate::error::*;
use indexsort::{sort_slice, search};
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
        inner.tables.extend(
            to_add.iter().map(|t| {
                add_total_size += t.table_size() as u64;
                add_total_stale_size += t.stale_data_size() as u64;
                t.clone()
            })
        );

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


    pub(super) fn get_table_for_key(&self, key: impl KeyExt) -> Option<Vec<Table>> {
        let inner = self.inner.read();
        if self.level == 0 {
            // For level 0, we need to check every table. Remember to make a copy as s.tables may change
		    // once we exit this function, and we don't want to lock s.tables while seeking in tables.
		    // CAUTION: Reverse the tables.
            let mut out = inner.tables.clone();
            out.reverse();
            return Some(out);
        }

        // For level >= 1, we can do a binary search as key range does not overlap.
        let idx = search(inner.tables.len(), | i| {
            inner.tables[i].biggest().as_key_ref() >= key.as_key_ref()
        });

        if idx >= inner.tables.len() {
            // Given key is strictly > than every element we have.
            return None;
        }

        Some(vec![inner.tables[idx].clone()])
    }


    /// Returns value for a given key or the key after that. If not found, return nil.
    pub(super) fn get(&self, key: impl KeyExt) -> Result<Value> {
        todo!()
    }

    pub(super) fn get_tables(&self) -> Vec<Table> {
        todo!()
    }

    pub(super) fn overlapping_tables<K1, K2>(&self, kr: ) -> (usize, usize) {

    }
}