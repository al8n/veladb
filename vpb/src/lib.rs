#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(docsrs, allow(unused_attributes))]
extern crate alloc;
use alloc::vec::Vec;

mod ty;
pub use ty::{kv::Kind, manifest_change::Operation, *};

pub use prost;
pub mod checksum {
    #[inline]
    pub fn calculate_checksum(_data: &[u8], algorithm: super::ChecksumAlgorithm) -> u64 {
        match algorithm {
            #[cfg(any(feature = "crc32", feature = "crc32-std"))]
            super::ChecksumAlgorithm::Crc32c => crc32(data),
            #[cfg(any(feature = "xxhash64", feature = "xxhash64-std"))] 
            super::ChecksumAlgorithm::XxHash64 => xxhash64(data),
            #[cfg(any(feature = "sea", feature = "sea-std"))]
            super::ChecksumAlgorithm::SeaHash => sea(data),
            #[allow(unreachable_patterns)]
            _ => panic!("Unsupported checksum algorithm: please enable one of checksum algorithm features (crc32, crc32-std, sea, sea-std, xxhash64, xxhash64-std)"),
        }
    }

    #[cfg(any(feature = "crc32", feature = "crc32-std"))]
    #[inline]
    pub fn crc32(data: &[u8]) -> u64 {
        crc32fast::hash(data) as u64
    }

    #[cfg(any(feature = "sea", feature = "sea-std"))]
    #[inline]
    pub fn sea(data: &[u8]) -> u64 {
        use core::hash::{BuildHasher, BuildHasherDefault, Hash, Hasher};
        use lazy_static::lazy_static;

        lazy_static! {
            static ref SEA: BuildHasherDefault<seahash::SeaHasher> =
                BuildHasherDefault::<seahash::SeaHasher>::default();
        }

        let mut h = SEA.build_hasher();
        data.hash(&mut h);
        h.finish()
    }

    #[cfg(any(feature = "xxhash64", feature = "xxhash64-std"))]
    #[inline]
    pub fn xxhash64(data: &[u8]) -> u64 {
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
        h.finish()
    }
}

pub trait Marshaller {
    fn marshal(&self) -> Vec<u8>;

    fn unmarshal(data: &[u8]) -> Result<Self, prost::DecodeError>
    where
        Self: Sized;
}

impl<T: prost::Message + Default> Marshaller for T {
    fn marshal(&self) -> Vec<u8> {
        self.encode_to_vec()
    }

    fn unmarshal(data: &[u8]) -> Result<Self, prost::DecodeError>
    where
        Self: Sized,
    {
        prost::Message::decode(data)
    }
}

macro_rules! impl_type {
    ($($ty: ty), +$(,)?) => {
        $(
            impl $ty {
                pub fn new() -> Self {
                    Self::default()
                }
            }
        )*
    };
}

impl_type! {
    Checksum, DataKey, ManifestChange, ManifestChangeSet, Match, Kv, KvList, BlockOffset, TableIndex,
}

#[derive(Copy, Clone, Default, Eq, PartialEq, Hash)]
pub struct InvalidChecksumAlgorithm;

impl core::fmt::Debug for InvalidChecksumAlgorithm {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "invalid checksum algorithm")
    }
}

impl core::fmt::Display for InvalidChecksumAlgorithm {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "invalid checksum algorithm")
    }
}

#[cfg(feature = "std")]
impl std::error::Error for InvalidChecksumAlgorithm {}

macro_rules! impl_checksum_algorithm_converter {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl TryFrom<$ty> for ChecksumAlgorithm {
                type Error = InvalidChecksumAlgorithm;

                fn try_from(val: $ty) -> Result<ChecksumAlgorithm, InvalidChecksumAlgorithm> {
                    match val {
                        0 => Ok(ChecksumAlgorithm::Crc32c),
                        1 => Ok(ChecksumAlgorithm::XxHash64),
                        2 => Ok(ChecksumAlgorithm::SeaHash),
                        _ => Err(InvalidChecksumAlgorithm),
                    }
                }
            }
        )*
    };
}

impl_checksum_algorithm_converter!(i8, i16, i32, i64, isize, i128, u8, u16, u32, u64, usize, u128);

#[derive(Copy, Clone, Default, Eq, PartialEq, Hash)]
pub struct InvalidManifestChangeOperation;

impl core::fmt::Debug for InvalidManifestChangeOperation {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "invalid manifest change operation")
    }
}

impl core::fmt::Display for InvalidManifestChangeOperation {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "invalid manifest change operation")
    }
}

#[cfg(feature = "std")]
impl std::error::Error for InvalidManifestChangeOperation {}

macro_rules! impl_operation_converter {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl TryFrom<$ty> for Operation {
                type Error = InvalidManifestChangeOperation;

                fn try_from(val: $ty) -> Result<Operation, InvalidManifestChangeOperation> {
                    match val {
                        0 => Ok(Operation::Create),
                        1 => Ok(Operation::Delete),
                        _ => Err(InvalidManifestChangeOperation),
                    }
                }
            }
        )*
    };
}

impl_operation_converter!(i8, i16, i32, i64, isize, i128, u8, u16, u32, u64, usize, u128);

#[derive(Copy, Clone, Default, Eq, PartialEq, Hash)]
pub struct InvalidKvKind;

impl core::fmt::Debug for InvalidKvKind {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "invalid key-value message kind")
    }
}

impl core::fmt::Display for InvalidKvKind {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "invalid key-value message kind")
    }
}

#[cfg(feature = "std")]
impl std::error::Error for InvalidKvKind {}

macro_rules! impl_kv_kind_converter {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl TryFrom<$ty> for Kind {
                type Error = InvalidKvKind;

                fn try_from(val: $ty) -> Result<Kind, InvalidKvKind> {
                    match val {
                        0 => Ok(Kind::Key),
                        1 => Ok(Kind::DataKey),
                        2 => Ok(Kind::File),
                        _ => Err(InvalidKvKind),
                    }
                }
            }
        )*
    };
}

impl_kv_kind_converter!(i8, i16, i32, i64, isize, i128, u8, u16, u32, u64, usize, u128);

#[derive(Copy, Clone, Default, Eq, PartialEq, Hash)]
pub struct InvalidEncryptionAlgorithm;

impl core::fmt::Debug for InvalidEncryptionAlgorithm {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "invalid encryption algorithm")
    }
}

impl core::fmt::Display for InvalidEncryptionAlgorithm {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "invalid encryption algorithm")
    }
}

#[cfg(feature = "std")]
impl std::error::Error for InvalidEncryptionAlgorithm {}

macro_rules! impl_encryption_algo_converter {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl TryFrom<$ty> for EncryptionAlgorithm {
                type Error = InvalidEncryptionAlgorithm;

                fn try_from(val: $ty) -> Result<EncryptionAlgorithm, InvalidEncryptionAlgorithm> {
                    match val {
                        0 => Ok(EncryptionAlgorithm::Aes),
                        _ => Err(InvalidEncryptionAlgorithm),
                    }
                }
            }
        )*
    };
}

impl_encryption_algo_converter!(i8, i16, i32, i64, isize, i128, u8, u16, u32, u64, usize, u128);

impl ManifestChange {
    #[inline]
    pub const fn new_create_change(
        id: u64,
        level: usize,
        key_id: u64,
        compression_ty: u32,
    ) -> Self {
        Self {
            id,
            op: Operation::Create as i32,
            level: level as u32,
            key_id,
            encryption_algo: EncryptionAlgorithm::Aes as i32,
            compression: compression_ty,
        }
    }

    #[inline]
    pub const fn new_delete_change(id: u64) -> Self {
        Self {
            id,
            op: Operation::Delete as i32,
            level: 0,
            key_id: 0,
            encryption_algo: 0,
            compression: 0,
        }
    }
}
