#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(
    feature = "nightly",
    feature(const_fn_floating_point_arithmetic),
    feature(generic_associated_types)
)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(docsrs, allow(unused_attributes))]
#[cfg(feature = "std")]
extern crate std;

#[cfg(all(feature = "std", feature = "metrics"))]
use std::collections::HashMap;

#[cfg(all(not(feature = "std"), feature = "metrics"))]
use hashbrown::HashMap;

extern crate alloc;

pub use vpb;
pub use zallocator;

pub mod cache;

pub mod error;

mod table;
pub use table::*;

pub mod bloom;

#[cfg(feature = "metrics")]
pub mod metrics;

#[cfg(feature = "std")]
mod sync {
    pub use core::sync::atomic::*;
    pub use triomphe::Arc;
}

pub use vela_utils::ref_counter::RefCounter;

fn binary_search<F: FnMut(isize) -> bool>(target: isize, mut op: F) -> isize {
    // Define f(-1) == false and f(n) == true.
    // Invariant: f(i-1) == false, f(j) == true.
    let (mut i, mut j) = (0, target);
    while i < j {
        let h = (((i + j) as usize) >> 1) as isize; // avoid overflow when computing h
                                                    // i ≤ h < j
        if !op(h) {
            i = h + 1; // preserves f(i-1) == false
        } else {
            j = h; // preserves f(j) == true
        }
    }

    // i == j, f(i-1) == false, and f(j) (= f(i)) == true  =>  answer is i.
    i
}
