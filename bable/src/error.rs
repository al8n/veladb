use alloc::string::String;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    ChecksumMismatch,
    EmptyBlock,
    BlockChecksumMismatch {
        table: String,
        block: usize,
    },
    BlockOffsetChecksumMismatch {
        table: String,
        block: usize,
        offset: usize,
    },
    InvalidChecksumLength,
    AllocateOverflow(zallocator::Overflow),
    EncryptError(vpb::encrypt::EncryptError),
    DecodeError(vpb::prost::DecodeError),
    BlockOutOfRange {
        num_offsets: u64,
        index: u64,
    },
    Compression(vpb::compression::Error),
    #[cfg(feature = "std")]
    IO(std::io::Error),
    #[cfg(feature = "std")]
    MmapError(fmmap::error::Error),
    #[cfg(feature = "std")]
    CacheError(stretto::CacheError),
    #[cfg(feature = "std")]
    InvalidFile(String),
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

impl From<vpb::compression::Error> for Error {
    fn from(e: vpb::compression::Error) -> Self {
        Error::Compression(e)
    }
}

impl From<vpb::prost::DecodeError> for Error {
    fn from(e: vpb::prost::DecodeError) -> Self {
        Error::DecodeError(e)
    }
}

#[cfg(feature = "std")]
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
            Error::BlockOutOfRange { num_offsets, index } => write!(f, "block out of range: {} not in range (0..{})", index, num_offsets),
            Error::InvalidChecksumLength => write!(f, "invalid checksum length: either the data is corrupted or the table options are incorrectly set"),
            Error::BlockChecksumMismatch { table, block } => write!(f, "block checksum mismatch: block {} in table {}", block, table),
            Error::BlockOffsetChecksumMismatch { table, block, offset } => write!(f, "block offset checksum mismatch: block {} in table {} at offset {}", block, table, offset),
            Error::Compression(e) => write!(f, "compression/decompression error: {}", e),
            Error::EmptyBlock => write!(f, "block size cannot be zero"),
            #[cfg(feature = "std")]
            Error::InvalidFile(name) => write!(f, "invalid file name: {name}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}
