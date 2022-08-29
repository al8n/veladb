#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(feature = "nightly", feature(const_fn_floating_point_arithmetic))]
#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "std")]
use std::collections::{hash_map::Entry, HashMap};

#[cfg(not(feature = "std"))]
use hashbrown::{hash_map::Entry, HashMap};

extern crate alloc;

pub use vpb;
pub use zallocator;

pub mod cache;

pub mod error;

mod table;
pub use table::*;
mod builder;
pub use builder::*;

pub mod bloom;
mod options;
pub use options::TableOptions;

#[cfg(feature = "metrics")]
pub mod metrics;

#[cfg(feature = "std")]
mod sync {
    pub use core::sync::atomic::*;
    pub use triomphe::Arc;
}

/// `RefCounter<T>` is a very simple wrapper, you can treat this like
///
/// - `triomphe::Arc` in `std`
/// - `alloc::rc::Rc` in `no_std` with `rc` feature enabled
#[derive(Debug)]
#[repr(transparent)]
pub struct RefCounter<T: ?Sized> {
    #[cfg(not(all(not(feature = "std"), feature = "rc")))]
    ptr: sync::Arc<T>,

    #[cfg(all(not(feature = "std"), feature = "rc"))]
    ptr: alloc::rc::Rc<T>,
}

impl<T> Clone for RefCounter<T> {
    fn clone(&self) -> Self {
        Self {
            ptr: self.ptr.clone(),
        }
    }
}

impl<T: ?Sized> core::ops::Deref for RefCounter<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.ptr
    }
}

impl<T> RefCounter<T> {
    #[inline]
    pub fn new(val: T) -> Self {
        Self {
            #[cfg(not(all(not(feature = "std"), feature = "rc")))]
            ptr: sync::Arc::new(val),
            #[cfg(all(not(feature = "std"), feature = "rc"))]
            ptr: alloc::rc::Rc::new(val),
        }
    }

    #[inline]
    pub fn count(ptr: &Self) -> usize {
        #[cfg(not(all(not(feature = "std"), feature = "rc")))]
        {
            sync::Arc::count(&ptr.ptr)
        }

        #[cfg(all(not(feature = "std"), feature = "rc"))]
        {
            alloc::rc::Rc::strong_count(&ptr.ptr)
        }
    }
}
