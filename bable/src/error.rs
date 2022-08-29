pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    ChecksumMismatch,
    AllocateOverflow(zallocator::Overflow),
    EncryptError(vpb::encrypt::EncryptError),
    DecodeError(vpb::prost::DecodeError),
    #[cfg(feature = "std")]
    IO(std::io::Error),
    #[cfg(feature = "std")]
    MmapError(fmmap::error::Error),
    #[cfg(feature = "std")]
    CacheError(stretto::CacheError),
}

#[cfg(feature = "std")]
impl From<fmmap::error::Error> for Error {
    fn from(err: fmmap::error::Error) -> Self {
        Error::MmapError(err)
    }
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

impl From<vpb::encrypt::EncryptError> for Error {
    fn from(e: vpb::encrypt::EncryptError) -> Self {
        Error::EncryptError(e)
    }
}

impl From<vpb::prost::DecodeError> for Error {
    fn from(e: vpb::prost::DecodeError) -> Self {
        Error::DecodeError(e)
    }
}

impl From<stretto::CacheError> for Error {
    fn from(e: stretto::CacheError) -> Self {
        Error::CacheError(e)
    }
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            Error::ChecksumMismatch => write!(f, "checksum mismatch"),
            Error::AllocateOverflow(o) => write!(f, "allocate overflow: {}", o),
            #[cfg(feature = "std")]
            Error::IO(e) => write!(f, "io error: {}", e),
            Error::EncryptError(e) => write!(f, "{}", e),
            #[cfg(feature = "std")]
            Error::MmapError(e) => write!(f, "mmap error: {}", e),
            Error::DecodeError(e) => write!(f, "decode error: {}", e),
            #[cfg(feature = "std")]
            Error::CacheError(e) => write!(f, "cache error: {}", e),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}
