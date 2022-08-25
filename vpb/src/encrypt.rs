#[cfg(any(feature = "aes", feature = "aes-std"))]
use aes::cipher::{KeyIvInit, StreamCipher, StreamCipherError};
#[cfg(any(feature = "aes", feature = "aes-std"))]
use aes::{Aes128, Aes192, Aes256};

pub const BLOCK_SIZE: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum EncryptionAlgorithm {
    None = 0,
    #[cfg(any(feature = "aes", feature = "aes-std"))]
    Aes = 1,
}

impl EncryptionAlgorithm {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            EncryptionAlgorithm::None => "None",
            #[cfg(any(feature = "aes", feature = "aes-std"))]
            EncryptionAlgorithm::Aes => "Aes",
        }
    }
}

impl EncryptionAlgorithm {
    #[inline]
    pub const fn is_none(&self) -> bool {
        matches!(self, EncryptionAlgorithm::None)
    }
}

#[derive(Debug, Clone)]
pub struct Encryption {
    algo: EncryptionAlgorithm,
    secret: kvstructs::bytes::Bytes,
}

impl Encryption {
    #[inline]
    pub const fn new() -> Self {
        Self {
            algo: EncryptionAlgorithm::None,
            secret: kvstructs::bytes::Bytes::new(),
        }
    }

    #[inline]
    pub const fn algorithm(&self) -> EncryptionAlgorithm {
        self.algo
    }

    #[inline]
    pub const fn is_none(&self) -> bool {
        self.algo.is_none()
    }

    #[inline]
    pub fn secret(&self) -> &[u8] {
        self.secret.as_ref()
    }

    /// Set the secret used to encrypt/decrypt the encrypted text.
    #[inline]
    pub fn set_secret(&mut self, secret: kvstructs::bytes::Bytes) {
        self.secret = secret;
    }
}

impl prost::Message for Encryption {
    #[allow(unused_variables)]
    fn encode_raw<B>(&self, buf: &mut B)
    where
        B: prost::bytes::BufMut,
    {
        if self.algo != EncryptionAlgorithm::default() {
            prost::encoding::int32::encode(1u32, &(self.algo as i32), buf);
        }
        if self.secret != b"" as &[u8] {
            prost::encoding::bytes::encode(2u32, &self.secret, buf);
        }
    }
    #[allow(unused_variables)]
    fn merge_field<B>(
        &mut self,
        tag: u32,
        wire_type: prost::encoding::WireType,
        buf: &mut B,
        ctx: prost::encoding::DecodeContext,
    ) -> ::core::result::Result<(), prost::DecodeError>
    where
        B: prost::bytes::Buf,
    {
        const STRUCT_NAME: &str = "Encryption";
        match tag {
            1u32 => {
                let value = &mut (self.algo as i32);
                prost::encoding::int32::merge(wire_type, value, buf, ctx).map_err(|mut error| {
                    error.push(STRUCT_NAME, "algo");
                    error
                })
            }
            2u32 => {
                let value = &mut self.secret;
                prost::encoding::bytes::merge(wire_type, value, buf, ctx).map_err(|mut error| {
                    error.push(STRUCT_NAME, "secret");
                    error
                })
            }
            _ => prost::encoding::skip_field(wire_type, tag, buf, ctx),
        }
    }
    #[inline]
    fn encoded_len(&self) -> usize {
        (if self.algo != EncryptionAlgorithm::default() {
            prost::encoding::int32::encoded_len(1u32, &(self.algo as i32))
        } else {
            0
        }) + (if self.secret != b"" as &[u8] {
            prost::encoding::bytes::encoded_len(2u32, &self.secret)
        } else {
            0
        })
    }
    fn clear(&mut self) {
        self.algo = EncryptionAlgorithm::default();
        self.secret.clear();
    }
}

/// AES-128 in CTR mode
#[cfg(any(feature = "aes", feature = "aes-std"))]
pub type Aes128Ctr = ctr::Ctr64BE<Aes128>;

/// AES-192 in CTR mode
#[cfg(any(feature = "aes", feature = "aes-std"))]
pub type Aes192Ctr = ctr::Ctr64BE<Aes192>;

/// AES-256 in CTR mode
#[cfg(any(feature = "aes", feature = "aes-std"))]
pub type Aes256Ctr = ctr::Ctr64BE<Aes256>;

#[cfg(any(feature = "aes", feature = "aes-std"))]
#[derive(Debug, Copy, Clone)]
pub enum AesError {
    InvalidLength(aes::cipher::InvalidLength),
    KeySizeError(usize),
    StreamCipherError(StreamCipherError),
}

#[cfg(any(feature = "aes", feature = "aes-std"))]
impl core::fmt::Display for AesError {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            AesError::KeySizeError(size) => write!(
                f,
                "aes: invalid key size {}, only supports 16, 24, 32 length for key",
                size
            ),
            AesError::StreamCipherError(err) => write!(f, "aes: {}", err),
            AesError::InvalidLength(e) => write!(f, "{}", e),
        }
    }
}

#[cfg(all(any(feature = "aes", feature = "aes-std"), feature = "std"))]
impl std::error::Error for AesError {}

#[derive(Debug, Copy, Clone)]
pub enum EncryptError {
    #[cfg(any(feature = "aes", feature = "aes-std"))]
    Aes(AesError),
    LengthMismatch {
        src: usize,
        dst: usize,
    },
    // #[cfg(feature = "std")]
    // IO(std::io::ErrorKind),
}

impl core::fmt::Display for EncryptError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            EncryptError::LengthMismatch { src, dst } => write!(
                f,
                "aes length mismatch: the length of source is {} and the length of destination {}",
                src, dst
            ),
            #[cfg(any(feature = "aes", feature = "aes-std"))]
            EncryptError::Aes(e) => write!(f, "{}", e), // EncryptError::IO(kd) => write!(f, "aes fail to write encrypt stream: {:?}", kd),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for EncryptError {}

/// AES encryption extensions
pub trait Encryptor {
    /// Encrypts self with IV.
    /// Can be used for both encryption and decryption.
    ///
    /// IV:
    ///     - `EncryptionAlgorithm::Aes`: IV is of AES block size.
    ///     - `EncryptionAlgorithm::None`: IV is ignored.
    fn encrypt(
        &mut self,
        key: &[u8],
        iv: &[u8],
        algo: EncryptionAlgorithm,
    ) -> Result<(), EncryptError>
    where
        Self: AsMut<[u8]>,
    {
        let data = self.as_mut();
        encrypt(data, key, iv, algo)
    }

    /// Encrypts self with IV to a new `Vec`.
    /// Can be used for both encryption and decryption.
    ///
    /// IV:
    ///     - `EncryptionAlgorithm::Aes`: IV is of AES block size.
    ///     - `EncryptionAlgorithm::None`: IV is ignored.
    fn encrypt_to_vec(
        &self,
        key: &[u8],
        iv: &[u8],
        algo: EncryptionAlgorithm,
    ) -> Result<Vec<u8>, EncryptError>
    where
        Self: AsRef<[u8]>,
    {
        let src = self.as_ref();
        encrypt_to_vec(src, key, iv, algo)
    }

    /// Encrypts self with IV to `dst`.
    /// Can be used for both encryption and decryption.
    ///
    /// IV:
    ///     - `EncryptionAlgorithm::Aes`: IV is of AES block size.
    ///     - `EncryptionAlgorithm::None`: IV is ignored.
    fn encrypt_to(
        &self,
        dst: &mut [u8],
        key: &[u8],
        iv: &[u8],
        algo: EncryptionAlgorithm,
    ) -> Result<(), EncryptError>
    where
        Self: AsRef<[u8]>,
    {
        let src = self.as_ref();
        encrypt_to(dst, src, key, iv, algo)
    }
}

impl<T> Encryptor for T {}

/// Encrypts data with IV.
/// Can be used for both encryption and decryption.
///
/// IV:
///     - `EncryptionAlgorithm::Aes`: IV is of AES block size.
///     - `EncryptionAlgorithm::None`: IV is ignored.
#[inline]
pub fn encrypt(
    data: &mut [u8],
    key: &[u8],
    iv: &[u8],
    algo: EncryptionAlgorithm,
) -> Result<(), EncryptError> {
    match algo {
        #[cfg(any(feature = "aes", feature = "aes-std"))]
        EncryptionAlgorithm::Aes => aes_encrypt_in(data, key, iv),
        _ => Ok(()),
    }
}

/// Encrypts src with IV to a new `Vec`.
/// Can be used for both encryption and decryption.
///
/// IV:
///     - `EncryptionAlgorithm::Aes`: IV is of AES block size.
///     - `EncryptionAlgorithm::None`: IV is ignored.
#[inline]
pub fn encrypt_to_vec(
    src: &[u8],
    key: &[u8],
    iv: &[u8],
    algo: EncryptionAlgorithm,
) -> Result<Vec<u8>, EncryptError> {
    let mut dst = src.to_vec();
    match algo {
        #[cfg(any(feature = "aes", feature = "aes-std"))]
        EncryptionAlgorithm::Aes => aes_encrypt_in(dst.as_mut(), key, iv).map(|_| dst),
        _ => Ok(dst),
    }
}

/// Encrypts `src` with IV to `dst`.
/// Can be used for both encryption and decryption.
///
/// IV:
///     - `EncryptionAlgorithm::Aes`: IV is of AES block size.
///     - `EncryptionAlgorithm::None`: IV is ignored.
#[inline]
pub fn encrypt_to(
    dst: &mut [u8],
    src: &[u8],
    key: &[u8],
    iv: &[u8],
    algo: EncryptionAlgorithm,
) -> Result<(), EncryptError> {
    if dst.len() != src.len() {
        return Err(EncryptError::LengthMismatch {
            src: src.len(),
            dst: dst.len(),
        });
    }
    dst.copy_from_slice(src);
    match algo {
        #[cfg(any(feature = "aes", feature = "aes-std"))]
        EncryptionAlgorithm::Aes => aes_encrypt_in(dst, key, iv),
        _ => Ok(()),
    }
}

#[cfg(any(feature = "aes", feature = "aes-std"))]
#[inline(always)]
fn aes_encrypt_in(dst: &mut [u8], key: &[u8], iv: &[u8]) -> Result<(), EncryptError> {
    let kl = key.len();
    match kl {
        16 => Aes128Ctr::new_from_slices(key, iv)
            .map_err(|e| EncryptError::Aes(AesError::InvalidLength(e)))?
            .try_apply_keystream(dst)
            .map_err(|e| EncryptError::Aes(AesError::StreamCipherError(e))),
        24 => Aes192Ctr::new_from_slices(key, iv)
            .map_err(|e| EncryptError::Aes(AesError::InvalidLength(e)))?
            .try_apply_keystream(dst)
            .map_err(|e| EncryptError::Aes(AesError::StreamCipherError(e))),
        32 => Aes256Ctr::new_from_slices(key, iv)
            .map_err(|e| EncryptError::Aes(AesError::InvalidLength(e)))?
            .try_apply_keystream(dst)
            .map_err(|e| EncryptError::Aes(AesError::StreamCipherError(e))),
        _ => Err(EncryptError::Aes(AesError::KeySizeError(kl))),
    }
}

/// generates IV.
#[cfg(any(feature = "aes", feature = "aes-std"))]
#[inline]
pub fn random_iv() -> [u8; BLOCK_SIZE] {
    #[cfg(feature = "aes-std")]
    {
        use rand::{thread_rng, Rng};
        let mut rng = thread_rng();
        rng.gen::<[u8; BLOCK_SIZE]>()
    }

    #[cfg(not(feature = "aes-std"))]
    {
        use rand::{rngs::OsRng, RngCore};
        let mut key = [0u8; BLOCK_SIZE];
        OsRng.fill_bytes(&mut key);
        key
    }
}

macro_rules! impl_encryption_algo_converter {
        ($($ty:ty),+ $(,)?) => {
            $(
                impl From<$ty> for EncryptionAlgorithm {
                    fn from(val: $ty) -> EncryptionAlgorithm {
                        match val {
                            #[cfg(any(feature = "aes", feature = "aes-std"))]
                            1 => EncryptionAlgorithm::Aes,
                            _ => EncryptionAlgorithm::None,
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
    fn test_encrypt() {
        let mut rng = thread_rng();
        let key = rng.gen::<[u8; 32]>();
        let iv = random_iv();

        let mut src = [0u8; 1024];
        rng.fill(&mut src);

        let mut dst = vec![0u8; 1024];
        encrypt_to(
            dst.as_mut_slice(),
            &src,
            &key,
            &iv,
            EncryptionAlgorithm::Aes,
        )
        .unwrap();

        let act = encrypt_to_vec(dst.as_slice(), &key, &iv, EncryptionAlgorithm::Aes).unwrap();
        assert_eq!(src.clone().to_vec(), act);

        let mut dst = vec![0u8; 1024];
        encrypt_to(
            dst.as_mut_slice(),
            &src,
            &key,
            &iv,
            EncryptionAlgorithm::None,
        )
        .unwrap();
        assert_eq!(dst.as_slice(), src.as_ref());

        let act = encrypt_to_vec(dst.as_slice(), &key, &iv, EncryptionAlgorithm::None).unwrap();
        assert_eq!(src.clone().to_vec(), act);
    }
}
