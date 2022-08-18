#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(docsrs, allow(unused_attributes))]
extern crate alloc;
use alloc::vec::Vec;

mod ty;
pub use ty::{kv::Kind, manifest_change::Operation, *};

pub use prost;

/// Encryption/Decryption
#[cfg(feature = "encryption")]
pub mod encrypt {
    use aes::cipher::{KeyIvInit, StreamCipher};
    use aes::{Aes128, Aes192, Aes256};

    pub const BLOCK_SIZE: usize = 16;

    /// AES-128 in CTR mode
    pub type Aes128Ctr = ctr::Ctr64BE<Aes128>;

    /// AES-192 in CTR mode
    pub type Aes192Ctr = ctr::Ctr64BE<Aes192>;

    /// AES-256 in CTR mode
    pub type Aes256Ctr = ctr::Ctr64BE<Aes256>;

    #[derive(Debug, Copy, Clone)]
    pub enum EncryptError {
        InvalidLength,
        LengthMismatch { src: usize, dst: usize },
        KeySizeError(usize),
        // #[cfg(feature = "std")]
        // IO(std::io::ErrorKind),
    }

    impl core::fmt::Display for EncryptError {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match self {
                EncryptError::InvalidLength => write!(f, "aes: invalid length when generate AES",),
                EncryptError::LengthMismatch { src, dst } => write!(
                    f,
                    "aes length mismatch: the length of source is {} and the length of destination {}",
                    src, dst
                ),
                EncryptError::KeySizeError(sz) => write!(f, "aes: invalid key size {}", sz),
                // EncryptError::IO(kd) => write!(f, "aes fail to write encrypt stream: {:?}", kd),
            }
        }
    }

    #[cfg(feature = "std")]
    impl std::error::Error for EncryptError {}

    /// AES encryption extensions
    pub trait EncryptExt: AsMut<[u8]> + AsRef<[u8]> {
        /// Encrypts self with IV.
        /// Can be used for both encryption and decryption. IV is of
        /// AES block size.
        fn encrypt(&mut self, key: &[u8], iv: &[u8]) -> Result<(), EncryptError> {
            let data = self.as_mut();
            encrypt_in(data, key, iv)
        }

        /// Encrypts self with IV to a new `Vec`.
        /// Can be used for both encryption and decryption. IV is of
        /// AES block size.
        fn encrypt_to_vec(&self, key: &[u8], iv: &[u8]) -> Result<Vec<u8>, EncryptError> {
            let src = self.as_ref();
            encrypt_to_vec(src, key, iv)
        }

        /// Encrypts self with IV to `dst`.
        /// Can be used for both encryption and decryption. IV is of
        /// AES block size.
        fn encrypt_to(&self, dst: &mut [u8], key: &[u8], iv: &[u8]) -> Result<(), EncryptError> {
            let src = self.as_ref();
            encrypt_to(dst, src, key, iv)
        }
    }

    impl<T: AsMut<[u8]> + AsRef<[u8]>> EncryptExt for T {}

    /// Encrypts data with IV.
    /// Can be used for both encryption and decryption. IV is of
    /// AES block size.
    #[inline]
    pub fn encrypt(data: &mut [u8], key: &[u8], iv: &[u8]) -> Result<(), EncryptError> {
        encrypt_in(data, key, iv)
    }

    /// Encrypts src with IV to a new `Vec`.
    /// Can be used for both encryption and decryption. IV is of
    /// AES block size.
    #[inline]
    pub fn encrypt_to_vec(src: &[u8], key: &[u8], iv: &[u8]) -> Result<Vec<u8>, EncryptError> {
        let mut dst = src.to_vec();
        encrypt_in(dst.as_mut(), key, iv).map(|_| dst)
    }

    /// Encrypts `src` with IV to `dst`.
    /// Can be used for both encryption and decryption. IV is of
    /// AES block size.
    #[inline]
    pub fn encrypt_to(
        dst: &mut [u8],
        src: &[u8],
        key: &[u8],
        iv: &[u8],
    ) -> Result<(), EncryptError> {
        if dst.len() != src.len() {
            return Err(EncryptError::LengthMismatch {
                src: src.len(),
                dst: dst.len(),
            });
        }
        dst.copy_from_slice(src);
        encrypt_in(dst, key, iv)
    }

    #[inline(always)]
    fn encrypt_in(dst: &mut [u8], key: &[u8], iv: &[u8]) -> Result<(), EncryptError> {
        let kl = key.len();
        match kl {
            16 => Aes128Ctr::new_from_slices(key, iv)
                .map_err(|_| EncryptError::InvalidLength)?
                .apply_keystream(dst),
            24 => Aes192Ctr::new_from_slices(key, iv)
                .map_err(|_| EncryptError::InvalidLength)?
                .apply_keystream(dst),
            32 => Aes256Ctr::new_from_slices(key, iv)
                .map_err(|_| EncryptError::InvalidLength)?
                .apply_keystream(dst),
            _ => return Err(EncryptError::KeySizeError(kl)),
        }
        Ok(())
    }

    /// generates IV.
    #[inline]
    pub fn random_iv() -> [u8; BLOCK_SIZE] {
        #[cfg(feature = "encryption-std")]
        {
            use rand::{thread_rng, Rng};
            let mut rng = thread_rng();
            rng.gen::<[u8; BLOCK_SIZE]>()
        }

        #[cfg(not(feature = "encryption-std"))]
        {
            use rand::{rngs::OsRng, RngCore};
            let mut key = [0u8; BLOCK_SIZE];
            OsRng.fill_bytes(&mut key);
            key
        }
    }

    #[cfg(test)]
    mod test {
        use super::*;
        use rand::{thread_rng, Rng};

        #[test]
        fn test_aes_block() {
            let mut rng = thread_rng();
            let key = rng.gen::<[u8; 32]>();
            let iv = random_iv();

            let mut src = [0u8; 1024];
            rng.fill(&mut src);

            let mut dst = vec![0u8; 1024];
            encrypt_to(dst.as_mut_slice(), &src, &key, &iv).unwrap();

            let act = encrypt_to_vec(dst.as_slice(), &key, &iv).unwrap();
            assert_eq!(src.clone().to_vec(), act);
        }
    }
}

/// Checksum calculation.
pub mod checksum {
    /// Calculates checksum for data using ct checksum type.
    #[inline]
    pub fn calculate_checksum(data: &[u8], algorithm: super::ChecksumAlgorithm) -> u64 {
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

    /// Validates the checksum for the data against the given expected checksum.
    #[inline]
    pub fn verify_checksum(
        data: &[u8],
        expected: super::Checksum,
    ) -> Result<bool, super::InvalidChecksumAlgorithm> {
        super::ChecksumAlgorithm::try_from(expected.algo)
            .map(|algo| calculate_checksum(data, algo) == expected.sum)
    }

    /// Validates the checksum for the data against the given expected checksum.
    ///
    /// # Panic
    /// The algorithm is not one of the supported checksum algorithms.
    #[inline]
    pub fn verify_checksum_unchecked(data: &[u8], expected: super::Checksum) -> bool {
        let algo =
            super::ChecksumAlgorithm::try_from(expected.algo).expect("Invalid checksum algorithm");
        calculate_checksum(data, algo) == expected.sum
    }

    /// Calculate crc32 checksum
    #[cfg(any(feature = "crc32", feature = "crc32-std"))]
    #[inline]
    pub fn crc32(data: &[u8]) -> u64 {
        crc32fast::hash(data) as u64
    }

    /// Calculate sea hash checksum
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

    /// Calculate xxhash64 checksum
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
