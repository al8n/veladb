#[cfg(feature = "std")]
mod standard;
#[cfg(feature = "std")]
pub use standard::*;

use super::error::*;

const VALUE_LOG_FILE_EXTENSION: &str = "vlog";

/// Size of vlog header.
///
/// ```text
/// +----------------+------------------+
/// | keyID(8 bytes) |  baseIV(12 bytes)|
/// +----------------+------------------+
/// ```
pub const VALUE_LOG_HEADER_SIZE: usize = 20;

/// The maximum size of the vlog file which can be created. Vlog Offset is of
/// `u32`, so limiting at `u32::MAX`.
const MAX_VALUE_LOG_SIZE: u32 = u32::MAX;
