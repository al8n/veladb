#[cfg(feature = "std")]
mod standard;
#[cfg(feature = "std")]
pub use standard::*;

const VALUE_LOG_FILE_EXTENSION: &str = "vlog";

/// Size of vlog header.
///
/// ```text
/// +----------------+------------------+
/// | keyID(8 bytes) |  baseIV(12 bytes)|
/// +----------------+------------------+
/// ```
pub const VALUE_LOG_HEADER_SIZE: usize = 20;
