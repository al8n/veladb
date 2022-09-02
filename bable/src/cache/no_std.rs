use lru::LruCache;
use core::cell::UnsafeCell;
use super::*;


#[derive(Clone)]
#[repr(transparent)]
pub struct IndexCache {
    inner: RefCounter<UnsafeCell<LruCache<u64, RefCounter<TableIndex>>>>,
}

impl IndexCache {
    #[inline]
    pub fn new(size: usize) -> IndexCache {
        Self {
            inner: RefCounter::new(UnsafeCell::new(LruCache::new(size))),
        }
    }

    /// Safety: this method will only be used in single thread feature
    #[inline]
    pub fn get(&self, k: &u64) -> Option<&RefCounter<TableIndex>> {
        unsafe { &mut *self.inner.get() }.get(k)
    }

    /// Safety: this method will only be used in single thread feature
    #[inline]
    pub fn insert(&self, k: u64, value: RefCounter<TableIndex>) {
        unsafe { &mut *self.inner.get() }.push(k, value); 
    }
}


#[derive(Clone)]
#[repr(transparent)]
pub struct BlockCache {
    inner: RefCounter<UnsafeCell<LruCache<Bytes, RefCounter<Block>>>>,
}

impl BlockCache {
    #[inline]
    pub fn new(size: usize) -> BlockCache {
        Self {
            inner: RefCounter::new(UnsafeCell::new(LruCache::new(size))),
        }
    }

    /// Safety: this method will only be used in single thread feature
    #[inline]
    pub fn get(&self, k: &[u8]) -> Option<&RefCounter<Block>> {
        unsafe { &mut *self.inner.get() }.get(k)
    }

    /// Safety: this method will only be used in single thread feature
    #[inline]
    pub fn insert(&self, k: Bytes, value: RefCounter<Block>) {
        unsafe { &mut *self.inner.get() }.push(k, value); 
    }
}