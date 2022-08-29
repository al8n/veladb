use crate::sync::{AtomicUsize, Ordering};
use crate::{Entry, HashMap};
use core::cell::UnsafeCell;

pub(crate) const DOES_NOT_HAVE_ALL: &str = "DOES_NOT_HAVE_ALL";
pub(crate) const DOES_NOT_HAVE_HIT: &str = "DOES_NOT_HAVE_HIT";

struct MapCell {
    inner: UnsafeCell<HashMap<&'static str, AtomicUsize>>,
}

impl MapCell {
    #[inline(always)]
    const fn new(inner: HashMap<&'static str, AtomicUsize>) -> Self {
        Self {
            inner: UnsafeCell::new(inner),
        }
    }

    /// Safety:
    ///
    /// - no remove happens in inner `HashMap<&'static str, AtomicUsize>`, so no use after free happens.
    /// - the value is atomic, so thread-safely update.
    #[inline(always)]
    fn get_inner_mut(&self) -> &mut HashMap<&'static str, AtomicUsize> {
        unsafe { &mut *self.inner.get() }
    }
}

unsafe impl Sync for MapCell {}
unsafe impl Send for MapCell {}

impl core::ops::Deref for MapCell {
    type Target = HashMap<&'static str, AtomicUsize>;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.inner.get() }
    }
}

lazy_static::lazy_static! {
    static ref NUM_BLOOM_HITS_ADD: MapCell = {
        let mut map = HashMap::with_capacity(10);
        map.insert(DOES_NOT_HAVE_ALL, AtomicUsize::new(0));
        map.insert(DOES_NOT_HAVE_HIT, AtomicUsize::new(0));
        MapCell::new(map)
    };
}

#[inline]
pub fn add_bloom_hits(key: &'static str, delta: usize) {
    match NUM_BLOOM_HITS_ADD.get_inner_mut().entry(key) {
        Entry::Occupied(ent) => {
            ent.get().fetch_add(delta, Ordering::SeqCst);
        }
        Entry::Vacant(ent) => {
            ent.insert(AtomicUsize::new(delta));
        }
    }
}

#[inline]
pub fn bloom_hits_map() -> &'static HashMap<&'static str, AtomicUsize> {
    &NUM_BLOOM_HITS_ADD
}
