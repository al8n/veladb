pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    EncryptError(vpb::encrypt::EncryptError),
    InvalidEncryptionKeyLength(usize),
    InvalidDataKeyID(u64),
    SanityError,
    SecrectMismatch,
    DecodeError(vpb::prost::DecodeError),
    #[cfg(feature = "std")]
    Mmap(fmmap::error::Error),
    #[cfg(feature = "std")]
    IO(std::io::Error),
    ChecksumMismatch,
    Truncate,
    TruncateNeeded {
        end_offset: u32,
        size: u32,
    },
    Stop,
    EOF,
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

#[cfg(feature = "std")]
impl From<fmmap::error::Error> for Error {
    fn from(e: fmmap::error::Error) -> Self {
        Error::Mmap(e)
    }
}

#[cfg(feature = "std")]
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::IO(e)
    }
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::EncryptError(e) => write!(f, "{}", e),
            Error::InvalidEncryptionKeyLength(len) => {
                write!(f, "invalid encryption key length {}", len)
            }
            Error::Mmap(e) => write!(f, "mmap: {}", e),
            Error::SanityError => write!(f, "sanity: error while reading sanity text"),
            Error::SecrectMismatch => write!(f, "secret: encryption key mismatch"),
            Error::ChecksumMismatch => {
                write!(f, "checksum: error while checking checksum for data key.")
            }
            Error::DecodeError(e) => write!(f, "decode: {}", e),
            Error::InvalidDataKeyID(e) => write!(f, "invalid data key id {}", e),
            Error::IO(e) => write!(f, "io: {}", e),
            Error::EOF => write!(f, "eof"),
            Error::Truncate => write!(f, "do truncate"),
            Error::Stop => write!(f, "stop iteration"),
            Error::TruncateNeeded { end_offset, size } => {
                write!(f, "end offset: {} < size: {}", end_offset, size)
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}
