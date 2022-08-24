pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    ChecksumMismatch,
    AllocateOverflow(zallocator::Overflow),
    #[cfg(feature = "std")]
    IO(std::io::Error),
}

#[cfg(feature = "std")]
impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::IO(err)
    }
}

impl From<zallocator::Overflow> for Error {
    fn from(e: zallocator::Overflow) -> Self {
        Error::AllocateOverflow(e)
    }
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            Error::ChecksumMismatch => write!(f, "checksum mismatch"),
            Error::AllocateOverflow(o) => write!(f, "allocate overflow: {}", o),
            #[cfg(feature = "std")]
            Error::IO(e) => write!(f, "io error: {}", e),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}
