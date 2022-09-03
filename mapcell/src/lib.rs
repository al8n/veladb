#![cfg_attr(not(feature = "std"), no_std)]
#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "std")]
use std::collections::{hash_map::Entry, HashMap};

#[cfg(not(feature = "std"))]
use hashbrown::{hash_map::Entry, HashMap};

use atomic::Atomic;
use core::cell::UnsafeCell;
use core::sync::atomic::*;

pub struct MapCell<K, V> {
    inner: UnsafeCell<HashMap<K, Atomic<V>>>,
    tag: &'static str,
}

impl<K: core::fmt::Debug, V: core::fmt::Debug + Copy> core::fmt::Debug for MapCell<K, V> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let map = unsafe { &*self.inner.get() };
        f.debug_struct(core::any::type_name::<Self>())
            .field("tag", &self.tag)
            .field("map", map)
            .finish()
    }
}

impl<K, V> MapCell<K, V> {
    /// Safety:
    ///
    /// - no remove happens in inner `HashMap`, so no use after free happens.
    /// - the value is atomic, so thread-safely update.
    #[allow(clippy::mut_from_ref)]
    #[inline(always)]
    fn get_inner_mut(&self) -> &mut HashMap<K, Atomic<V>> {
        unsafe { &mut *self.inner.get() }
    }

    /// Safety:
    ///
    /// - no remove happens in inner `HashMap`, so no use after free happens.
    #[inline(always)]
    fn get_inner(&self) -> &HashMap<K, Atomic<V>> {
        unsafe { &*self.inner.get() }
    }

    #[inline(always)]
    pub const fn tag(&self) -> &'static str {
        self.tag
    }
}

impl<K: core::hash::Hash + Eq, V> MapCell<K, V> {
    #[inline(always)]
    pub fn new(tag: &'static str, inner: HashMap<K, V>) -> Self {
        Self {
            inner: UnsafeCell::new(
                inner
                    .into_iter()
                    .map(|(k, v)| (k, Atomic::new(v)))
                    .collect(),
            ),
            tag,
        }
    }

    #[inline(always)]
    pub fn set(&self, k: K, v: V) {
        self.get_inner_mut().insert(k, Atomic::new(v));
    }
}

impl<K: core::hash::Hash + Eq, V: Copy> MapCell<K, V> {
    #[inline(always)]
    pub fn get<Q>(&self, key: &Q, ordering: Ordering) -> Option<V>
    where
        K: core::borrow::Borrow<Q>,
        Q: core::hash::Hash + Eq,
    {
        self.get_inner().get(key).map(|v| v.load(ordering))
    }

    #[inline(always)]
    pub fn update<Q>(&self, k: &Q, v: V, ordering: Ordering)
    where
        K: core::borrow::Borrow<Q>,
        Q: core::hash::Hash + Eq,
    {
        if let Some(val) = self.get_inner_mut().get(k) {
            val.store(v, ordering)
        }
    }
}

impl<K: core::hash::Hash + Eq, V: Copy + core::ops::Add<Output = V>> MapCell<K, V> {
    #[inline(always)]
    pub fn fetch_add(&self, key: K, delta: V) {
        match self.get_inner_mut().entry(key) {
            Entry::Occupied(ent) => {
                let _ = ent
                    .get()
                    .fetch_update(Ordering::Release, Ordering::Acquire, |v| Some(v.add(delta)));
            }
            Entry::Vacant(ent) => {
                ent.insert(Atomic::new(delta));
            }
        }
    }
}

impl<K: core::hash::Hash + Eq, V: Copy + core::ops::Sub<Output = V>> MapCell<K, V> {
    #[inline(always)]
    pub fn fetch_sub(&self, key: K, delta: V) {
        match self.get_inner_mut().entry(key) {
            Entry::Occupied(ent) => {
                let _ = ent
                    .get()
                    .fetch_update(Ordering::Release, Ordering::Acquire, |v| Some(v.sub(delta)));
            }
            Entry::Vacant(ent) => {
                ent.insert(Atomic::new(delta));
            }
        }
    }
}

unsafe impl<K, V> Sync for MapCell<K, V> {}
unsafe impl<K, V> Send for MapCell<K, V> {}
