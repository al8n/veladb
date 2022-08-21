#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(docsrs, allow(unused_attributes))]
extern crate alloc;
use alloc::vec::Vec;

mod ty;
pub use ty::{
    kv::Kind, manifest_change::Operation, BlockOffset, CompressionAlgorithm, DataKey, Kv, KvList,
    ManifestChange, ManifestChangeSet, Match, TableIndex,
};

pub use prost;

/// Compression
pub use compression::Compression;

pub mod compression {
    use alloc::vec::Vec;

    impl Copy for crate::ty::Compression {}

    /// Compression specifies how a block should be compressed.
    #[derive(Debug)]
    pub struct Compression {
        /// compression algorithm
        pub algo: crate::ty::CompressionAlgorithm,
        /// only for zstd, <= 0 use default(3) compression level, 1 - 21 uses the exact level of zstd compression level, >=22 use the largest compression level supported by zstd.
        pub level: i32,
    }

    impl Default for Compression {
        fn default() -> Self {
            Self::new()
        }
    }

    impl Compression {
        pub const fn new() -> Self {
            Self {
                algo: super::CompressionAlgorithm::None,
                level: 0,
            }
        }
    }

    impl prost::Message for Compression {
        #[allow(unused_variables)]
        fn encode_raw<B>(&self, buf: &mut B)
        where
            B: ::prost::bytes::BufMut,
        {
            if self.algo != super::CompressionAlgorithm::default() {
                ::prost::encoding::int32::encode(1u32, &(self.algo as i32), buf);
            }
            if self.level != 0i32 {
                ::prost::encoding::int32::encode(2u32, &self.level, buf);
            }
        }

        #[allow(unused_variables)]
        fn merge_field<B>(
            &mut self,
            tag: u32,
            wire_type: ::prost::encoding::WireType,
            buf: &mut B,
            ctx: ::prost::encoding::DecodeContext,
        ) -> ::core::result::Result<(), ::prost::DecodeError>
        where
            B: ::prost::bytes::Buf,
        {
            const STRUCT_NAME: &str = "Compression";
            match tag {
                1u32 => {
                    let value = &mut (self.algo as i32);
                    ::prost::encoding::int32::merge(wire_type, value, buf, ctx).map_err(
                        |mut error| {
                            error.push(STRUCT_NAME, "algo");
                            error
                        },
                    )
                }
                2u32 => {
                    let value = &mut self.level;
                    ::prost::encoding::int32::merge(wire_type, value, buf, ctx).map_err(
                        |mut error| {
                            error.push(STRUCT_NAME, "level");
                            error
                        },
                    )
                }
                _ => ::prost::encoding::skip_field(wire_type, tag, buf, ctx),
            }
        }

        #[inline]
        fn encoded_len(&self) -> usize {
            (if self.algo != super::CompressionAlgorithm::default() {
                ::prost::encoding::int32::encoded_len(1u32, &(self.algo as i32))
            } else {
                0
            }) + (if self.level != 0i32 {
                ::prost::encoding::int32::encoded_len(2u32, &self.level)
            } else {
                0
            })
        }

        #[inline]
        fn clear(&mut self) {
            self.algo = super::CompressionAlgorithm::default();
            self.level = 0i32;
        }
    }

    /// Compression/Decompression Error
    pub enum Error {
        /// Snappy error
        #[cfg(feature = "snappy")]
        Snappy(snap::Error),
        /// Zstd error
        #[cfg(feature = "zstd")]
        Zstd(std::io::Error),
        /// Lz4 error
        #[cfg(any(feature = "lz4", feature = "lz4-std"))]
        Lz4(Lz4Error),
        /// This error occurs when the given buffer is too small to contain the
        /// maximum possible compressed bytes or the total number of decompressed
        /// bytes.
        BufferTooSmall {
            /// The size of the given output buffer.
            given: u64,
            /// The minimum size of the output buffer.
            min: u64,
        },
    }

    impl core::fmt::Debug for Error {
        fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
            match self {
                #[cfg(feature = "snappy")]
                Error::Snappy(e) => write!(f, "{:?}", e),
                #[cfg(feature = "zstd")]
                Error::Zstd(e) => write!(f, "{:?}", e),
                #[cfg(any(feature = "lz4", feature = "lz4-std"))]
                Error::Lz4(e) => write!(f, "{:?}", e),
                Error::BufferTooSmall { given, min } => {
                    write!(f, "BufferTooSmall {{given {}, min {}}}", given, min)
                }
            }
        }
    }

    impl core::fmt::Display for Error {
        fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
            match self {
                #[cfg(feature = "snappy")]
                Error::Snappy(e) => write!(f, "snappy error: {}", e),
                #[cfg(feature = "zstd")]
                Error::Zstd(e) => write!(f, "zstd error: {}", e),
                #[cfg(any(feature = "lz4", feature = "lz4-std"))]
                Error::Lz4(e) => write!(f, "{}", e),
                Error::BufferTooSmall { given, min } => {
                    write!(f, "buffer too small: given {}, min {}", given, min)
                }
            }
        }
    }

    #[cfg(feature = "std")]
    impl std::error::Error for Error {}

    #[cfg(any(feature = "lz4", feature = "lz4-std"))]
    pub enum Lz4Error {
        Compression(lz4_flex::block::CompressError),
        Decompression(lz4_flex::block::DecompressError),
    }

    #[cfg(any(feature = "lz4", feature = "lz4-std"))]
    impl core::fmt::Debug for Lz4Error {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match self {
                Lz4Error::Compression(e) => write!(f, "lz4 error: {:?}", e),
                Lz4Error::Decompression(e) => write!(f, "lz4 error: {:?}", e),
            }
        }
    }

    #[cfg(any(feature = "lz4", feature = "lz4-std"))]
    impl core::fmt::Display for Lz4Error {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match self {
                Lz4Error::Compression(e) => write!(f, "lz4 error: {}", e),
                Lz4Error::Decompression(e) => write!(f, "lz4 error: {}", e),
            }
        }
    }

    #[cfg(all(any(feature = "lz4", feature = "lz4-std"), feature = "std"))]
    impl std::error::Error for Lz4Error {}

    pub trait Compressor {
        #[inline]
        fn compress_to(&self, dst: &mut [u8], cmp: Compression) -> Result<usize, Error>
        where
            Self: AsRef<[u8]>,
        {
            compress_to(self.as_ref(), dst, cmp)
        }

        #[inline]
        fn compress_into_vec(&self, cmp: Compression) -> Result<Vec<u8>, Error>
        where
            Self: AsRef<[u8]>,
        {
            compress_into_vec(self.as_ref(), cmp)
        }

        #[inline]
        fn decompress_to(&self, dst: &mut [u8], cmp: Compression) -> Result<usize, Error>
        where
            Self: AsRef<[u8]>,
        {
            decompress_to(self.as_ref(), dst, cmp)
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

    impl<T> Compressor for T {}

    #[inline]
    pub fn compress_to(data: &[u8], dst: &mut [u8], cmp: Compression) -> Result<usize, Error> {
        match cmp.algo {
            #[cfg(feature = "snappy")]
            super::CompressionAlgorithm::Snappy => snap::raw::Encoder::new()
                .compress(data, dst)
                .map_err(Error::Snappy),
            #[cfg(feature = "zstd")]
            super::CompressionAlgorithm::Zstd => {
                let range = zstd_compression::compression_level_range();
                match cmp.level {
                    lvl if range.contains(&lvl) => {
                        zstd_compression::bulk::compress_to_buffer(data, dst, lvl)
                            .map_err(Error::Zstd)
                    }
                    lvl if lvl <= 0 => zstd_compression::bulk::compress_to_buffer(data, dst, 0)
                        .map_err(Error::Zstd),
                    _ => {
                        zstd_compression::bulk::compress_to_buffer(data, dst, range.last().unwrap())
                            .map_err(Error::Zstd)
                    }
                }
            }
            #[cfg(any(feature = "lz4", feature = "lz4-std"))]
            super::CompressionAlgorithm::Lz4 => lz4_flex::compress_into(data, &mut dst[4..])
                .map(|size| {
                    dst[..4].copy_from_slice((size as u32).to_le_bytes().as_ref());
                    size
                })
                .map_err(|e| Error::Lz4(Lz4Error::Compression(e))),
            _ => {
                if data.len() > dst.len() {
                    return Err(Error::BufferTooSmall {
                        given: dst.len() as u64,
                        min: data.len() as u64,
                    });
                }
                dst[..data.len()].copy_from_slice(data);
                Ok(data.len())
            }
        }
    }

    #[inline]
    pub fn compress_into_vec(data: &[u8], cmp: Compression) -> Result<Vec<u8>, Error> {
        match cmp.algo {
            #[cfg(feature = "snappy")]
            super::CompressionAlgorithm::Snappy => snap::raw::Encoder::new()
                .compress_vec(data)
                .map_err(Error::Snappy),
            #[cfg(feature = "zstd")]
            super::CompressionAlgorithm::Zstd => {
                let range = zstd_compression::compression_level_range();
                match cmp.level {
                    lvl if range.contains(&lvl) => zstd_compression::encode_all(data, lvl),
                    lvl if lvl <= 0 => zstd_compression::encode_all(data, 0),
                    _ => zstd_compression::encode_all(data, range.last().unwrap()),
                }
                .map_err(Error::Zstd)
            }
            #[cfg(any(feature = "lz4", feature = "lz4-std"))]
            super::CompressionAlgorithm::Lz4 => Ok(lz4_flex::compress_prepend_size(data)),
            _ => Ok(data.to_vec()),
        }
    }

    #[inline]
    pub fn decompress_to(data: &[u8], dst: &mut [u8], cmp: Compression) -> Result<usize, Error> {
        match cmp.algo {
            #[cfg(feature = "snappy")]
            super::CompressionAlgorithm::Snappy => snap::raw::Decoder::new()
                .decompress(data, dst)
                .map_err(Error::Snappy),
            #[cfg(feature = "zstd")]
            super::CompressionAlgorithm::Zstd => {
                zstd_compression::bulk::decompress_to_buffer(data, dst).map_err(Error::Zstd)
            }
            #[cfg(any(feature = "lz4", feature = "lz4-std"))]
            super::CompressionAlgorithm::Lz4 => lz4_flex::decompress_into(&data[4..], dst)
                .map_err(|e| Error::Lz4(Lz4Error::Decompression(e))),
            _ => {
                if data.len() > dst.len() {
                    return Err(Error::BufferTooSmall {
                        given: dst.len() as u64,
                        min: data.len() as u64,
                    });
                }
                dst[..data.len()].copy_from_slice(data);
                Ok(data.len())
            }
        }
    }

    #[inline]
    pub fn decompress_into_vec(data: &[u8], cmp: Compression) -> Result<Vec<u8>, Error> {
        match cmp.algo {
            #[cfg(feature = "snappy")]
            super::CompressionAlgorithm::Snappy => snap::raw::Decoder::new()
                .decompress_vec(data)
                .map_err(Error::Snappy),
            #[cfg(feature = "zstd")]
            super::CompressionAlgorithm::Zstd => {
                zstd_compression::decode_all(data).map_err(Error::Zstd)
            }
            #[cfg(any(feature = "lz4", feature = "lz4-std"))]
            super::CompressionAlgorithm::Lz4 => lz4_flex::decompress_size_prepended(data)
                .map_err(|e| Error::Lz4(Lz4Error::Decompression(e))),
            _ => Ok(data.to_vec()),
        }
    }

    #[inline]
    pub fn max_encoded_len(data: &[u8], cmp: Compression) -> usize {
        match cmp.algo {
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
                lz4_flex::block::get_maximum_output_size(data.len()) + core::mem::size_of::<u32>()
            }
            _ => data.len(),
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
                            #[cfg(any(feature = "lz4", feature = "lz4-std"))]
                            3 => super::CompressionAlgorithm::Lz4,
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
    pub trait Encryptor {
        /// Encrypts self with IV.
        /// Can be used for both encryption and decryption. IV is of
        /// AES block size.
        fn encrypt(&mut self, key: &[u8], iv: &[u8]) -> Result<(), EncryptError>
        where
            Self: AsMut<[u8]>,
        {
            let data = self.as_mut();
            encrypt_in(data, key, iv)
        }

        /// Encrypts self with IV to a new `Vec`.
        /// Can be used for both encryption and decryption. IV is of
        /// AES block size.
        fn encrypt_to_vec(&self, key: &[u8], iv: &[u8]) -> Result<Vec<u8>, EncryptError>
        where
            Self: AsRef<[u8]>,
        {
            let src = self.as_ref();
            encrypt_to_vec(src, key, iv)
        }

        /// Encrypts self with IV to `dst`.
        /// Can be used for both encryption and decryption. IV is of
        /// AES block size.
        fn encrypt_to(&self, dst: &mut [u8], key: &[u8], iv: &[u8]) -> Result<(), EncryptError>
        where
            Self: AsRef<[u8]>,
        {
            let src = self.as_ref();
            encrypt_to(dst, src, key, iv)
        }
    }

    impl<T> Encryptor for T {}

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
    impl super::Checksum {
        pub const fn new() -> Self {
            Self { algo: 0, sum: 0 }
        }
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

        /// Validates the checksum for the data against the given expected checksum.
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

/// Proto buf encode/decode
pub trait Marshaller {
    /// Encode to proto buf
    fn marshal(&self) -> Vec<u8>
    where
        Self: prost::Message + Default,
    {
        self.encode_to_vec()
    }

    /// Decode from proto buf
    fn unmarshal(data: &[u8]) -> Result<Self, prost::DecodeError>
    where
        Self: prost::Message + Default + Sized,
    {
        prost::Message::decode(data)
    }
}

impl<T> Marshaller for T {}

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
    DataKey, ManifestChange, ManifestChangeSet, Match, Kv, KvList, BlockOffset, TableIndex,
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
            encryption_algo: 0, // we only have one encryption algorithm currently
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
            encryption_algo: 0, // we only have one encryption algorithm currently
            compression: 0,
        }
    }
}
