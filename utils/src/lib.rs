#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(docsrs, allow(unused_attributes))]

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "ref_counter")]
extern crate alloc;
#[cfg(feature = "ref_counter")]
pub mod ref_counter;

#[cfg(feature = "map_cell")]
pub mod map_cell;
