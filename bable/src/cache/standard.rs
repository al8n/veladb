use stretto::{Cache, CacheCallback, Coster, DefaultKeyBuilder, KeyBuilder, UpdateValidator};
use vpb::{kvstructs::bytes::Bytes, TableIndex};

use crate::{error::*, table::Block, RefCounter};

#[doc(hidden)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NoopIndex;

impl UpdateValidator for NoopIndex {
    type Value = RefCounter<TableIndex>;

    #[inline(always)]
    fn should_update(&self, _prev: &Self::Value, _curr: &Self::Value) -> bool {
        true
    }
}

impl Coster for NoopIndex {
    type Value = RefCounter<TableIndex>;

    #[inline(always)]
    fn cost(&self, _val: &Self::Value) -> i64 {
        0
    }
}

impl CacheCallback for NoopIndex {
    type Value = RefCounter<TableIndex>;

    #[inline(always)]
    fn on_exit(&self, _val: Option<Self::Value>) {}
}

#[doc(hidden)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct IndexKeyBuilder;

impl KeyBuilder for IndexKeyBuilder {
    type Key = u64;

    #[inline(always)]
    fn hash_index(&self, key: &Self::Key) -> u64 {
        *key
    }
}

#[derive(Clone)]
#[repr(transparent)]
pub struct IndexCache(
    RefCounter<
        Cache<u64, RefCounter<TableIndex>, IndexKeyBuilder, NoopIndex, NoopIndex, NoopIndex>,
    >,
);

impl IndexCache {
    pub fn new(num_counters: usize, max_cost: i64) -> Result<Self> {
        Cache::builder(num_counters, max_cost)
            .set_callback(NoopIndex)
            .set_coster(NoopIndex)
            .set_update_validator(NoopIndex)
            .set_key_builder(IndexKeyBuilder)
            .finalize()
            .map(|c| Self(RefCounter::new(c)))
            .map_err(From::from)
    }
}

impl core::ops::Deref for IndexCache {
    type Target =
        Cache<u64, RefCounter<TableIndex>, IndexKeyBuilder, NoopIndex, NoopIndex, NoopIndex>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[doc(hidden)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NoopBlock;

impl UpdateValidator for NoopBlock {
    type Value = RefCounter<Block>;

    #[inline(always)]
    fn should_update(&self, _prev: &Self::Value, _curr: &Self::Value) -> bool {
        true
    }
}

impl Coster for NoopBlock {
    type Value = RefCounter<Block>;

    #[inline(always)]
    fn cost(&self, _val: &Self::Value) -> i64 {
        0
    }
}

impl CacheCallback for NoopBlock {
    type Value = RefCounter<Block>;

    #[inline(always)]
    fn on_exit(&self, _val: Option<Self::Value>) {}
}

#[doc(hidden)]
pub struct BlockKeyBuilder {
    sea: core::hash::BuildHasherDefault<vpb::checksum::SeaHasher>,
    xx: vpb::checksum::Xxh64Builder,
}

impl core::fmt::Debug for BlockKeyBuilder {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("BlockKeyBuilder").finish()
    }
}

impl Default for BlockKeyBuilder {
    fn default() -> Self {
        use rand::{thread_rng, Rng};
        let mut rng = thread_rng();
        let seed = rng.gen::<u64>();
        Self {
            sea: Default::default(),
            xx: vpb::checksum::Xxh64Builder::new(seed),
        }
    }
}

impl KeyBuilder for BlockKeyBuilder {
    type Key = Bytes;

    #[inline]
    fn hash_index(&self, key: &Self::Key) -> u64 {
        use core::hash::{BuildHasher, Hash, Hasher};

        let mut s = self.sea.build_hasher();
        key.hash(&mut s);
        s.finish()
    }

    #[inline]
    fn hash_conflict(&self, key: &Self::Key) -> u64 {
        use core::hash::{BuildHasher, Hash, Hasher};
        let mut x = self.xx.build_hasher();
        key.hash(&mut x);
        x.finish()
    }
}

#[derive(Clone)]
pub struct BlockCache(
    RefCounter<Cache<Bytes, RefCounter<Block>, BlockKeyBuilder, NoopBlock, NoopBlock, NoopBlock>>,
);

impl BlockCache {
    pub fn new(num_counters: usize, max_cost: i64) -> Result<Self> {
        Cache::builder(num_counters, max_cost)
            .set_callback(NoopBlock)
            .set_coster(NoopBlock)
            .set_update_validator(NoopBlock)
            .set_key_builder(BlockKeyBuilder::default())
            .finalize()
            .map(|c| Self(RefCounter::new(c)))
            .map_err(From::from)
    }
}

impl core::ops::Deref for BlockCache {
    type Target = Cache<Bytes, RefCounter<Block>, BlockKeyBuilder, NoopBlock, NoopBlock, NoopBlock>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
