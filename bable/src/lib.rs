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

pub use vela_utils;
pub use vpb;
pub use vpb::kvstructs;
pub use vpb::kvstructs::bytes;
pub use vpb::prost;
pub use zallocator;

pub mod cache;

pub mod error;

pub use stretto::Histogram;

mod table;
pub use table::*;

pub mod bloom;

#[cfg(feature = "metrics")]
pub mod metrics;

mod sync {
    pub use core::sync::atomic::*;
}

pub use vela_utils::ref_counter::RefCounter;
