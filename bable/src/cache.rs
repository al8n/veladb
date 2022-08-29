use vpb::TableIndex;

#[cfg(not(feature = "std"))]
mod no_std;
#[cfg(not(feature = "std"))]
pub use no_std::*;
#[cfg(feature = "std")]
mod standard;
#[cfg(feature = "std")]
pub use standard::*;
