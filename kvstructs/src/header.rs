use crate::{
    binary_put_uvariant_to_buf, binary_put_uvariant_to_bufmut, binary_put_uvariant_to_vec,
    binary_uvarint,
};
use alloc::vec::Vec;
use bytes::{BufMut, Bytes, BytesMut};

/// Maximum possible size of the header. The maximum size of header struct will be 18 but the
/// maximum size of variant encoded header will be 21.
pub const MAX_HEADER_SIZE: usize = 21;

/// Header is used in value log as a header before Entry.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[repr(C)]
pub struct Header {
    pub(crate) meta: u8,
    pub(crate) user_meta: u8,
    pub(crate) k_len: u32,
    pub(crate) v_len: u32,
    pub(crate) expires_at: u64,
}

impl Default for Header {
    fn default() -> Self {
        Self::new()
    }
}

impl Header {
    /// Create a new header.
    pub const fn new() -> Self {
        Self {
            meta: 0,
            user_meta: 0,
            k_len: 0,
            v_len: 0,
            expires_at: 0,
        }
    }

    /// Encodes the header into `Vec<u8>`. The provided `Vec<u8>` should be at least 5 bytes. The
    /// function will panic if out `Vec<u8>` isn't large enough to hold all the values.
    /// The encoded header looks like
    ///
    /// ```text
    /// +------+----------+------------+--------------+-----------+
    ///
    /// | Meta | UserMeta | Key Length | Value Length | ExpiresAt |
    ///
    /// +------+----------+------------+--------------+-----------+
    /// ```
    pub fn encode_to_vec(&self) -> (usize, Vec<u8>) {
        let mut buf = Vec::with_capacity(MAX_HEADER_SIZE);
        buf.push(self.meta);
        buf.push(self.user_meta);
        let mut index = 2;

        index += binary_put_uvariant_to_vec(&mut buf, self.k_len as u64);
        index += binary_put_uvariant_to_vec(&mut buf, self.v_len as u64);
        index += binary_put_uvariant_to_vec(&mut buf, self.expires_at);
        (index, buf)
    }

    /// Encodes the header into `Bytes`. The provided `Bytes` should be at least 5 bytes. The
    /// function will panic if out `Bytes` isn't large enough to hold all the values.
    /// The encoded header looks like
    ///
    /// ```text
    /// +------+----------+------------+--------------+-----------+
    ///
    /// | Meta | UserMeta | Key Length | Value Length | ExpiresAt |
    ///
    /// +------+----------+------------+--------------+-----------+
    /// ```
    pub fn encode_to_bytes(&self) -> (usize, Bytes) {
        let mut buf = BytesMut::with_capacity(MAX_HEADER_SIZE);
        buf.put_u8(self.meta);
        buf.put_u8(self.user_meta);
        let mut index = 2;

        index += binary_put_uvariant_to_bufmut(&mut buf, self.k_len as u64);
        index += binary_put_uvariant_to_bufmut(&mut buf, self.v_len as u64);
        index += binary_put_uvariant_to_bufmut(&mut buf, self.expires_at);
        (index, buf.freeze())
    }

    /// Encodes the header into `Bytes`. The provided `Bytes` should be at least 5 bytes. The
    /// function will panic if out `Bytes` isn't large enough to hold all the values.
    /// The encoded header looks like
    ///
    /// ```text
    /// +------+----------+------------+--------------+-----------+
    ///
    /// | Meta | UserMeta | Key Length | Value Length | ExpiresAt |
    ///
    /// +------+----------+------------+--------------+-----------+
    /// ```
    pub fn encode(&self, buf: &mut [u8]) -> usize {
        buf[0] = self.meta;
        buf[1] = self.user_meta;
        let mut index = 2;

        index += binary_put_uvariant_to_buf(&mut buf[index..], self.k_len as u64);
        index += binary_put_uvariant_to_buf(&mut buf[index..], self.v_len as u64);
        index += binary_put_uvariant_to_buf(&mut buf[index..], self.expires_at);
        index
    }

    /// Decode Header from byte slice, returns Header and number of bytes read
    pub fn decode(data: &[u8]) -> (usize, Self) {
        let mut index = 2;
        let (k_len, ctr) = binary_uvarint(&data[index..]);
        index += ctr;

        let (v_len, ctr) = binary_uvarint(&data[index..]);
        index += ctr;

        let (expires_at, ctr) = binary_uvarint(&data[index..]);
        (
            index + ctr,
            Self {
                k_len: k_len as u32,
                v_len: v_len as u32,
                expires_at,
                meta: data[0],
                user_meta: data[1],
            },
        )
    }

    /// Decode Header from Cursor<>, returns Header and number of bytes read
    #[cfg(feature = "std")]
    pub fn decode_reader(
        reader: &mut impl ByteReader,
        hash_buf: &mut BytesMut,
    ) -> std::io::Result<(usize, Self)> {
        use crate::binary_read_and_put_uvarint;

        let start = reader.position();
        let meta = reader.read_byte()?;
        hash_buf.put_u8(meta);
        let user_meta = reader.read_byte()?;
        hash_buf.put_u8(user_meta);

        let k_len = binary_read_and_put_uvarint(reader, hash_buf)?;
        let v_len = binary_read_and_put_uvarint(reader, hash_buf)?;
        let expires_at = binary_read_and_put_uvarint(reader, hash_buf)?;
        let h_len = reader.position() - start;
        let h = Header {
            k_len: k_len as u32,
            v_len: v_len as u32,
            expires_at,
            meta,
            user_meta,
        };

        Ok((h_len as usize, h))
    }

    /// update the data of the header according to the provided byte slice.
    /// Returns the number of bytes read.
    pub fn update(&mut self, data: &[u8]) -> usize {
        self.meta = data[0];
        self.user_meta = data[1];
        let mut index = 2;

        let (k_len, ctr) = binary_uvarint(&data[index..]);
        self.k_len = k_len as u32;
        index += ctr;

        let (v_len, ctr) = binary_uvarint(&data[index..]);
        self.v_len = v_len as u32;
        index += ctr;

        let (expires_at, ctr) = binary_uvarint(&data[index..]);
        self.expires_at = expires_at;
        index + ctr
    }

    /// Get the value length
    #[inline]
    pub fn get_value_len(&self) -> u32 {
        self.v_len
    }

    /// Set the value length
    #[inline]
    pub fn set_value_len(mut self, vlen: u32) -> Self {
        self.v_len = vlen;
        self
    }

    /// Get the key length
    #[inline]
    pub fn get_key_len(&self) -> u32 {
        self.k_len
    }

    /// Set the key length
    #[inline]
    pub fn set_key_len(mut self, klen: u32) -> Self {
        self.k_len = klen;
        self
    }

    /// Get the meta
    #[inline]
    pub fn get_meta(&self) -> u8 {
        self.meta
    }

    /// Set the meta
    #[inline]
    pub fn set_meta(mut self, meta: u8) -> Self {
        self.meta = meta;
        self
    }

    /// Get the user meta
    #[inline]
    pub fn get_user_meta(&self) -> u8 {
        self.user_meta
    }

    /// Set the user meta
    #[inline]
    pub fn set_user_meta(mut self, user_meta: u8) -> Self {
        self.user_meta = user_meta;
        self
    }

    /// Get the expires_at
    #[inline]
    pub fn get_expires_at(&self) -> u64 {
        self.expires_at
    }

    /// Set the expires_at
    #[inline]
    pub fn set_expires_at(mut self, expires_at: u64) -> Self {
        self.expires_at = expires_at;
        self
    }
}

cfg_std! {
    use std::io::Read;

    ///
    pub trait ByteReader {
        /// Read one byte and advance the current position
        fn read_byte(&mut self) -> std::io::Result<u8>;
        /// Returns the current position
        fn position(&self) -> u64;
    }

    macro_rules! impl_byte_reader_for_cursor {
        ($($ty: ty),+ $(,)?) => {
            $(
                impl ByteReader for std::io::Cursor<$ty> {
                    #[inline]
                    fn read_byte(&mut self) -> std::io::Result<u8> {
                        let mut buf = [0; 1];
                        self.read_exact(&mut buf).map(|_| buf[0])
                    }

                    #[inline]
                    fn position(&self) -> u64 {
                        self.position()
                    }
                }
            )*
        };
    }

    impl_byte_reader_for_cursor! {
        Box<[u8]>,
        &[u8],
        &mut [u8],
        Vec<u8>,
        &Vec<u8>,
        &mut Vec<u8>
    }
}
