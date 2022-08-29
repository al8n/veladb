#[cfg(any(feature = "sea", feature = "sea-std"))]
pub use seahash::*;

#[cfg(any(feature = "xxhash64", feature = "xxhash64-std"))]
pub use xxhash_rust::xxh64::*;

#[derive(Clone, PartialEq, Eq, Hash, ::prost::Message)]
pub struct Checksum {
    /// For storing type of Checksum algorithm used
    #[prost(enumeration = "ChecksumAlgorithm", tag = "1")]
    pub algo: i32,
    #[prost(uint64, tag = "2")]
    pub sum: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum ChecksumAlgorithm {
    Crc32c = 0,
    #[cfg(any(feature = "xxhash64", feature = "xxhash64-std"))]
    XxHash64 = 1,
    #[cfg(any(feature = "sea", feature = "sea-std"))]
    SeaHash = 2,
}

impl ChecksumAlgorithm {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub const fn as_str_name(&self) -> &'static str {
        match self {
            ChecksumAlgorithm::Crc32c => "CRC32C",
            #[cfg(any(feature = "xxhash64", feature = "xxhash64-std"))]
            ChecksumAlgorithm::XxHash64 => "XXHash64",
            #[cfg(any(feature = "sea", feature = "sea-std"))]
            ChecksumAlgorithm::SeaHash => "SeaHash",
        }
    }
}

impl Copy for Checksum {}

impl Checksum {
    /// The size of the checksum value in memory
    pub const ENCODED_SIZE: usize = core::mem::size_of::<u64>() + core::mem::size_of::<u32>();

    pub const fn new() -> Self {
        Self { algo: 0, sum: 0 }
    }
}

macro_rules! impl_checksum_algorithm_converter {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl From<$ty> for super::ChecksumAlgorithm {
                fn from(val: $ty) -> super::ChecksumAlgorithm {
                    match val {
                        #[cfg(any(feature = "xxhash64", feature = "xxhash64-std"))]
                        1 => super::ChecksumAlgorithm::XxHash64,
                        #[cfg(any(feature = "sea", feature = "sea-std"))]
                        2 => super::ChecksumAlgorithm::SeaHash,
                        _ => super::ChecksumAlgorithm::Crc32c,
                    }
                }
            }
        )*
    };
}

impl_checksum_algorithm_converter!(i8, i16, i32, i64, isize, i128, u8, u16, u32, u64, usize, u128);

pub trait Checksumer {
    /// Calculates checksum of `data`.
    fn checksum(&self, algorithm: super::ChecksumAlgorithm) -> super::Checksum
    where
        Self: AsRef<[u8]>,
    {
        calculate_checksum(self.as_ref(), algorithm)
    }

    /// Validates the checksum for the data against the given expected checksum. Return `true` if the checksum is valid.
    fn verify_checksum(&self, expected: u64, algo: super::ChecksumAlgorithm) -> bool
    where
        Self: AsRef<[u8]>,
    {
        verify_checksum(self.as_ref(), expected, algo)
    }
}

impl<T> Checksumer for T {}

/// Calculates checksum for data using ct checksum type.
#[inline]
pub fn calculate_checksum(data: &[u8], algorithm: super::ChecksumAlgorithm) -> super::Checksum {
    match algorithm {
        #[cfg(any(feature = "xxhash64", feature = "xxhash64-std"))]
        super::ChecksumAlgorithm::XxHash64 => xxhash64(data),
        #[cfg(any(feature = "sea", feature = "sea-std"))]
        super::ChecksumAlgorithm::SeaHash => sea(data),
        _ => crc32(data),
    }
}

/// Validates the checksum for the data against the given expected checksum. Return `true` if the checksum is valid.
#[inline]
pub fn verify_checksum(data: &[u8], expected: u64, algo: super::ChecksumAlgorithm) -> bool {
    calculate_checksum(data, algo).sum == expected
}

/// Calculate crc32 checksum
#[inline]
pub fn crc32(data: &[u8]) -> super::Checksum {
    super::Checksum {
        algo: super::ChecksumAlgorithm::Crc32c as i32,
        sum: crc32fast::hash(data) as u64,
    }
}

/// Calculate sea hash checksum
#[cfg(any(feature = "sea", feature = "sea-std"))]
#[inline]
pub fn sea(data: &[u8]) -> super::Checksum {
    use core::hash::{BuildHasher, BuildHasherDefault, Hash, Hasher};
    use lazy_static::lazy_static;

    lazy_static! {
        static ref SEA: BuildHasherDefault<seahash::SeaHasher> =
            BuildHasherDefault::<seahash::SeaHasher>::default();
    }

    let mut h = SEA.build_hasher();
    data.hash(&mut h);

    super::Checksum {
        algo: super::ChecksumAlgorithm::SeaHash as i32,
        sum: h.finish(),
    }
}

/// Calculate xxhash64 checksum
#[cfg(any(feature = "xxhash64", feature = "xxhash64-std"))]
#[inline]
pub fn xxhash64(data: &[u8]) -> super::Checksum {
    use core::hash::{BuildHasher, Hash, Hasher};
    use lazy_static::lazy_static;

    lazy_static! {
        static ref XXH64: xxhash_rust::xxh64::Xxh64Builder = {
            #[cfg(feature = "xxhash64-std")]
            {
                use rand::{thread_rng, Rng};
                let mut rng = thread_rng();
                let seed = rng.gen::<u64>();
                xxhash_rust::xxh64::Xxh64Builder::new(seed)
            }
            #[cfg(not(feature = "xxhash64-std"))]
            {
                use rand::{rngs::OsRng, RngCore};
                let mut key = [0u8; 8];
                OsRng.fill_bytes(&mut key);
                let seed = OsRng.next_u64();
                xxhash_rust::xxh64::Xxh64Builder::new(seed)
            }
        };
    }

    let mut h = XXH64.build_hasher();
    data.hash(&mut h);
    super::Checksum {
        algo: super::ChecksumAlgorithm::XxHash64 as i32,
        sum: h.finish(),
    }
}
