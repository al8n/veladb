use bable::kvstructs::ValuePointer;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    #[cfg(feature = "std")]
    Any(Box<dyn std::error::Error + Send + Sync>),
    CorruptedLogFile,
    EmptyValue,
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
    ChecksumMismatch {
        expected: u32,
        actual: u32,
    },
    Truncate,
    AlreadyMarkedDeletion(u32),
    LogFileNotFound(u32),
    InvalidValuePointer {
        current: u32,
        pointer: u32,
    },
    /// Returned if a call for value log GC doesn't result in a log file rewrite.
    NoRewrite,

    /// Returned if a value log GC is called either while another GC is running, or
    /// after DB::Close has been called.
    Rejected,
    TruncateNeeded {
        end_offset: u32,
        size: u32,
    },
    Stop,
    EOF,
    MaxValueLogSize {
        estimated_offset: u64,
        max_value_log_size: u32,
    },
    InvalidRead {
        len: usize,
        range: core::ops::Range<usize>,
    },
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
            #[cfg(feature = "std")]
            Error::Mmap(e) => write!(f, "mmap: {}", e),
            Error::SanityError => write!(f, "sanity: error while reading sanity text"),
            Error::SecrectMismatch => write!(f, "secret: encryption key mismatch"),
            Error::ChecksumMismatch { expected, actual } => {
                write!(f, "checksum mismatch: expected {expected}, actual {actual}")
            }
            Error::DecodeError(e) => write!(f, "decode: {}", e),
            Error::InvalidDataKeyID(e) => write!(f, "invalid data key id {}", e),
            #[cfg(feature = "std")]
            Error::IO(e) => write!(f, "io: {}", e),
            Error::EOF => write!(f, "eof"),
            Error::Truncate => write!(f, "do truncate"),
            Error::Stop => write!(f, "stop iteration"),
            Error::TruncateNeeded { end_offset, size } => {
                write!(f, "end offset: {} < size: {}", end_offset, size)
            }
            Error::AlreadyMarkedDeletion(fid) => {
                write!(f, "value log file already marked for deletion fid: {fid}")
            }
            Error::NoRewrite => write!(f, "Value log GC attempt didn't result in any cleanup"),
            Error::Rejected => write!(f, "Value log GC request rejected"),
            #[cfg(feature = "std")]
            Error::Any(e) => write!(f, "wisc: {e}"),
            Error::CorruptedLogFile => write!(f, "wisc: corrupted log file"),
            Error::LogFileNotFound(fid) => write!(f, "file with ID: {fid} not found"),
            Error::InvalidValuePointer {current, pointer} => write!(f, "invalid value pointer offset: {pointer} greater than current offset: {current}"),
            Error::InvalidRead { len, range } => write!(f, "invalid read: len: {len} read at: [{}..{}]", range.start, range.end),
            Error::MaxValueLogSize { estimated_offset, max_value_log_size } => write!(f, "wisc: request size offset {estimated_offset} is bigger than maximum offset {max_value_log_size}"),
            Error::EmptyValue => todo!(),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}
