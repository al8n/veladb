#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(docsrs, allow(unused_attributes))]
extern crate alloc;
use alloc::vec::Vec;

mod ty;
pub use ty::{kv::Kind, manifest_change::Operation, *};

pub use prost;

/// Compression
pub mod compression {
    use alloc::vec::Vec;

    impl Copy for super::Compression {}

    #[derive(Debug)]
    pub struct Compression {
        pub algo: super::CompressionAlgorithm,
        pub level: i32,
    }

    impl Compression {
        pub const fn new() -> Self {
            Self {
                algo: super::CompressionAlgorithm::None,
                level: 0,
            }
        }

        const fn to_pb_type(&self) -> super::Compression {
            super::Compression {
                algo: self.algo as i32,
                level: self.level,
            }
        }
    }

    impl prost::Message for Compression {
        fn encode_raw<B>(&self, buf: &mut B)
        where
            B: prost::bytes::BufMut,
            Self: Sized,
        {
            self.to_pb_type().encode_raw(buf)
        }

        fn merge_field<B>(
            &mut self,
            tag: u32,
            wire_type: prost::encoding::WireType,
            buf: &mut B,
            ctx: prost::encoding::DecodeContext,
        ) -> Result<(), prost::DecodeError>
        where
            B: prost::bytes::Buf,
            Self: Sized,
        {
            self.to_pb_type().merge_field(tag, wire_type, buf, ctx)
        }

        fn encoded_len(&self) -> usize {
            self.to_pb_type().encoded_len()
        }

        fn clear(&mut self) {
            self.to_pb_type().clear()
        }
    }

    #[derive(Copy, Clone, PartialEq, Eq, Hash)]
    pub enum Error {
        /// The data is too large to be compressed.
        Oversize(u64),
        InvalidFormat,
    }

    impl core::fmt::Debug for Error {
        fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
            match self {
                Error::Oversize(size) => write!(f, "Error::Oversize({size})"),
                Error::InvalidFormat => write!(f, "Error::InvalidFormat"),
            }
        }
    }

    impl core::fmt::Display for Error {
        fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
            match self {
                Error::Oversize(size) => {
                    write!(f, "The total number of bytes to compress exceeds {size}.")
                }
                Error::InvalidFormat => write!(f, "The data is not in the correct format."),
            }
        }
    }

    #[cfg(feature = "std")]
    impl std::error::Error for Error {}

    pub trait Compressor {
        #[inline]
        fn compress_into_vec(&self, cmp: Compression) -> Result<Vec<u8>, Error>
        where
            Self: AsRef<[u8]>,
        {
            compress_into_vec(self.as_ref(), cmp)
        }

        #[inline]
        fn decompress_into_vec(&self, cmp: Compression) -> Result<Vec<u8>, Error>
        where
            Self: AsRef<[u8]>,
        {
            decompress_into_vec(self.as_ref(), cmp)
        }

        #[inline]
        fn max_encoded_len(&self, cmp: Compression) -> usize
        where
            Self: AsRef<[u8]>,
        {
            max_encoded_len(self.as_ref(), cmp)
        }
    }

    impl<T: AsRef<[u8]>> Compressor for T {}

    #[inline]
    pub fn compress_into_vec(data: &[u8], cmp: Compression) -> Result<Vec<u8>, Error> {
        match cmp.algo {
            super::CompressionAlgorithm::None => Ok(data.to_vec()),
            #[cfg(feature = "snappy")]
            super::CompressionAlgorithm::Snappy => snap::raw::Encoder::new()
                .compress_vec(data)
                .map_err(|_| Error::Oversize((2 ^ 32) - 1)),
            #[cfg(feature = "zstd")]
            super::CompressionAlgorithm::Zstd => {
                let range = zstd_compression::compression_level_range();
                match cmp.level {
                    lvl if range.contains(&lvl) => zstd_compression::encode_all(data, lvl),
                    lvl if lvl <= 0 => Ok(data.to_vec()),
                    _ => zstd_compression::encode_all(data, range.last().unwrap()),
                }
                .map_err(|_| Error::InvalidFormat)
            }
            #[cfg(any(feature = "lz4", feature = "lz4-std"))]
            super::CompressionAlgorithm::Lz4 => Ok(lz4_flex::compress_prepend_size(data)),
        }
    }

    #[inline]
    pub fn decompress_into_vec(data: &[u8], cmp: Compression) -> Result<Vec<u8>, Error> {
        match cmp.algo {
            super::CompressionAlgorithm::None => Ok(data.to_vec()),
            #[cfg(feature = "snappy")]
            super::CompressionAlgorithm::Snappy => {
                snap::raw::Decoder::new().decompress_vec(data).map_err(|e| {
                    // TODO: Polish error handle
                    match e {
                        snap::Error::TooBig { .. } => Error::Oversize((2 ^ 32) - 1),
                        _ => Error::InvalidFormat,
                    }
                })
            }
            #[cfg(feature = "zstd")]
            super::CompressionAlgorithm::Zstd => {
                let range = zstd_compression::compression_level_range();
                match cmp.level {
                    v if range.contains(&v) => zstd_compression::decode_all(data),
                    v if v <= 0 => Ok(data.to_vec()),
                    _ => zstd_compression::decode_all(data),
                }
                .map_err(|_| Error::InvalidFormat)
            }
            #[cfg(any(feature = "lz4", feature = "lz4-std"))]
            super::CompressionAlgorithm::Lz4 => {
                lz4_flex::decompress_size_prepended(data).map_err(|_| Error::InvalidFormat)
            }
        }
    }

    #[inline]
    pub fn max_encoded_len(data: &[u8], cmp: Compression) -> usize {
        match cmp.algo {
            super::CompressionAlgorithm::None => data.len(),
            #[cfg(feature = "snappy")]
            super::CompressionAlgorithm::Snappy => snap::raw::max_compress_len(data.len()),
            #[cfg(feature = "zstd")]
            super::CompressionAlgorithm::Zstd => {
                let low_limit = 128 << 10; // 128 kb
                let src_size = data.len();
                let margin = if src_size < low_limit {
                    (low_limit - src_size) >> 11
                } else {
                    0
                };
                src_size + (src_size >> 8) + margin
            }
            #[cfg(any(feature = "lz4", feature = "lz4-std"))]
            super::CompressionAlgorithm::Lz4 => {
                lz4_flex::block::get_maximum_output_size(data.len())
            }
        }
    }

    macro_rules! impl_compression_algo_converter {
        ($($ty:ty),+ $(,)?) => {
            $(
                impl From<$ty> for super::CompressionAlgorithm {
                    fn from(val: $ty) -> super::CompressionAlgorithm {
                        match val {
                            #[cfg(feature = "snappy")]
                            1 => super::CompressionAlgorithm::Snappy,
                            #[cfg(feature = "zstd")]
                            2 => super::CompressionAlgorithm::Zstd,
                            _ => super::CompressionAlgorithm::None,
                        }
                    }
                }
            )*
        };
    }

    impl_compression_algo_converter!(
        i8, i16, i32, i64, isize, i128, u8, u16, u32, u64, usize, u128
    );
}

/// Encryption/Decryption
#[cfg(feature = "encryption")]
pub use ty::EncryptionAlgorithm;

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
    pub trait Encryptor: AsMut<[u8]> + AsRef<[u8]> {
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

    impl<T: AsMut<[u8]> + AsRef<[u8]>> Encryptor for T {}

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
                impl TryFrom<$ty> for super::EncryptionAlgorithm {
                    type Error = InvalidEncryptionAlgorithm;

                    fn try_from(val: $ty) -> Result<super::EncryptionAlgorithm, InvalidEncryptionAlgorithm> {
                        match val {
                            0 => Ok(super::EncryptionAlgorithm::Aes),
                            _ => Err(InvalidEncryptionAlgorithm),
                        }
                    }
                }
            )*
        };
    }

    impl_encryption_algo_converter!(i8, i16, i32, i64, isize, i128, u8, u16, u32, u64, usize, u128);

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
#[cfg(any(
    feature = "crc32",
    feature = "crc32-std",
    feature = "xxhash64",
    feature = "xxhash64-std",
    feature = "sea",
    feature = "sea-std"
))]
pub use ty::{Checksum, ChecksumAlgorithm};

#[cfg(any(
    feature = "crc32",
    feature = "crc32-std",
    feature = "xxhash64",
    feature = "xxhash64-std",
    feature = "sea",
    feature = "sea-std"
))]
pub mod checksum {
    impl Copy for super::Checksum {}

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
                impl TryFrom<$ty> for super::ChecksumAlgorithm {
                    type Error = InvalidChecksumAlgorithm;

                    fn try_from(val: $ty) -> Result<super::ChecksumAlgorithm, InvalidChecksumAlgorithm> {
                        match val {
                            0 => Ok(super::ChecksumAlgorithm::Crc32c),
                            1 => Ok(super::ChecksumAlgorithm::XxHash64),
                            2 => Ok(super::ChecksumAlgorithm::SeaHash),
                            _ => Err(InvalidChecksumAlgorithm),
                        }
                    }
                }
            )*
        };
    }

    impl_checksum_algorithm_converter!(
        i8, i16, i32, i64, isize, i128, u8, u16, u32, u64, usize, u128
    );

    pub trait Checksumer {
        /// Calculates checksum of `data`.
        fn checksum(&self, algorithm: super::ChecksumAlgorithm) -> super::Checksum
        where
            Self: AsRef<[u8]>,
        {
            calculate_checksum(self.as_ref(), algorithm)
        }

        fn verify_checksum(&self, expected: u64, algo: super::ChecksumAlgorithm) -> bool
        where
            Self: AsRef<[u8]>,
        {
            verify_checksum(self.as_ref(), expected, algo)
        }
    }

    impl<T: AsRef<[u8]>> Checksumer for T {}

    /// Calculates checksum for data using ct checksum type.
    #[inline]
    pub fn calculate_checksum(data: &[u8], algorithm: super::ChecksumAlgorithm) -> super::Checksum {
        match algorithm {
            #[cfg(any(feature = "crc32", feature = "crc32-std"))]
            super::ChecksumAlgorithm::Crc32c => crc32(data),
            #[cfg(any(feature = "xxhash64", feature = "xxhash64-std"))]
            super::ChecksumAlgorithm::XxHash64 => xxhash64(data),
            #[cfg(any(feature = "sea", feature = "sea-std"))]
            super::ChecksumAlgorithm::SeaHash => sea(data),
        }
    }

    /// Validates the checksum for the data against the given expected checksum.
    #[inline]
    pub fn verify_checksum(data: &[u8], expected: u64, algo: super::ChecksumAlgorithm) -> bool {
        calculate_checksum(data, algo).sum == expected
    }

    /// Calculate crc32 checksum
    #[cfg(any(feature = "crc32", feature = "crc32-std"))]
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
