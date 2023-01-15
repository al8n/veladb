use kvstructs::{KeyExt, KeyRange};
use parking_lot::RwLock;
use std::{collections::HashSet, sync::Arc};

use crate::levels::CompactDef;

pub(crate) struct LevelCompactStatus<L, R> {
    ranges: Vec<KeyRange<L, R>>,
    del_size: i64,
}

impl<L, R> LevelCompactStatus<L, R> {
    pub(crate) fn new() -> Self {
        Self {
            ranges: Vec::new(),
            del_size: 0,
        }
    }
}

impl<L: KeyExt, R: KeyExt> LevelCompactStatus<L, R> {
    pub(crate) fn overlaps_with(&mut self, range: &KeyRange<L, R>) -> bool {
        self.ranges.iter().any(|r| r.overlaps_with(range))
    }

    pub(crate) fn remove(&mut self, dst: &KeyRange<L, R>) -> bool {
        let mut found = false;
        self.ranges.retain(|r| {
            if !r.eq(dst) {
                true
            } else {
                found = true;
                false
            }
        });
        found
    }
}

#[derive(Clone)]
#[repr(transparent)]
pub(crate) struct CompactStatus<L, R> {
    inner: Arc<RwLock<CompactStatusInner<L, R>>>,
}

struct CompactStatusInner<L, R> {
    levels: Vec<LevelCompactStatus<L, R>>,
    tables: HashSet<u64>,
}

impl<L: KeyExt, R: KeyExt> CompactStatus<L, R> {
    pub(crate) fn overlaps_with(&self, level: usize, this: &KeyRange<L, R>) -> bool {
        self.inner.read().levels[level].overlaps_with(this)
    }

    pub(crate) fn del_size(&self, level: usize, dst: KeyRange<L, R>) -> i64 {
        self.inner.read().levels[level].del_size
    }

    pub(crate) fn compare_and_add(&self, _r: ThisAndNextLevelRLocked, cd: &CompactDef) -> bool {
        let mut inner = self.inner.write();
        let this_level = &mut inner.levels[cd.this_level.level];
        let next_level = &mut inner.levels[cd.next_level.level];
        if this_level.overlaps_with(&cd.this) {
            return false;
        }
        if next_level.overlaps_with(&cd.next) {
            return false;
        }

        // Check whether this level really needs compaction or not. Otherwise, we'll end up
        // running parallel compactions for the same level.
        // Update: We should not be checking size here. Compaction priority     already did the size checks.
        // Here we should just be executing the wish of others.
        this_level.ranges.push(cd.this_range);
        next_level.ranges.push(cd.next_range);
        this_level.del_size += cd.this_size;
        for t in cd.top.iter().chain(cd.bot.iter()) {
            inner.tables.insert(t.id());
        }
        true
    }

    pub(crate) fn remove(&self, cd: &CompactDef) {
        let mut inner = self.inner.write();

        let tl = cd.this_level.level;
        assert!(
            tl < inner.levels.len(),
            "Got level {}. Max levels: {}",
            tl,
            inner.levels.len()
        );

        let this_level = &mut inner.levels[cd.this_level.level];
        let next_level = &mut inner.levels[cd.next_level.level];

        this_level.del_size -= cd.this_size;
        let mut found = this_level.remove(&cd.this_range);
        // The following check makes sense only if we're compacting more than one
        // table. In case of the max level, we might rewrite a single table to
        // remove stale data.

        if cd.this_level != cd.next_level && !cd.next_range.is_empty() {
            found = next_level.remove(&cd.next_range) && found;
        }

        if !found {
            let this = &cd.this_range;
            let next = &cd.next_range;
            // TODO: tracing
            #[cfg(feature = "tracing")]
            {}
        }

        for t in cd.top.iter().chain(cd.bot.iter()) {
            let ok = inner.tables.remove(&t.id());
            assert!(ok);
        }
    }
}

struct ThisAndNextLevelRLocked;
