pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    ChecksumMismatch,
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            Error::ChecksumMismatch => write!(f, "checksum mismatch"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}
