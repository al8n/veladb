use crate::raw_value_pointer::RawValuePointer;
use crate::value_enc::EncodedValue;
use crate::{
    binary_uvarint, binary_uvarint_allocate, put_binary_uvariant_to_vec, EXPIRATION_OFFSET,
    META_OFFSET, USER_META_OFFSET,
};
use alloc::borrow::Cow;
use alloc::boxed::Box;
use alloc::rc::Rc;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use bytes::{Buf, BufMut, Bytes, BytesMut};
use core::mem;
use core::slice::from_raw_parts;

/// Max size of a value information. 1 for meta, 1 for user meta, 8 for expires_at
const MAX_VALUE_INFO_SIZE: usize = mem::size_of::<u8>() * 2 + mem::size_of::<u64>();

/// Value represents the value info that can be associated with a key, but also the internal
/// Meta field. The data in the Value is not mutable.
///
/// # Design for Value
///
/// **Note:**
/// 1. `version` field will not be encoded, it is a helper field.
/// 2. `expiration` field will be encoded as uvarient, which means after encoded, the size of
/// this field is less or equal to 8 bytes.
///
/// ```text
/// +----------+-----------------+--------------------+--------------------+--------------------+
/// |   meta   |    user meta    |     expiration     |      version       |        data        |
/// +----------+-----------------+--------------------+--------------------+--------------------+
/// |  1 byte  |      1 byte     |      8 bytes       |      8 bytes       |       n bytes      |
/// +----------+-----------------+--------------------+--------------------+--------------------+
/// ```
#[derive(Default, Debug, Eq, PartialEq, Ord, PartialOrd, Clone, Hash)]
#[repr(C)]
pub struct Value {
    pub(crate) meta: u8,
    pub(crate) user_meta: u8,
    pub(crate) expires_at: u64,
    pub(crate) version: u64, // This field is not serialized. Only for internal usage.
    pub(crate) value: Bytes,
}

impl From<Value> for Bytes {
    fn from(v: Value) -> Self {
        let mut b = BytesMut::with_capacity(MAX_VALUE_INFO_SIZE + v.value.len());
        b.put_u8(v.meta);
        b.put_u8(v.user_meta);
        b.put_u64(v.expires_at);
        b.extend(v.value);
        b.freeze()
    }
}

impl Value {
    /// Returns an empty value
    #[inline]
    pub const fn new() -> Self {
        Self {
            meta: 0,
            user_meta: 0,
            expires_at: 0,
            version: 0,
            value: Bytes::new(),
        }
    }

    /// Returns a value with the given meta, user meta, expires_at, version and data.
    #[inline]
    pub const fn with_all_fields(
        meta: u8,
        user_meta: u8,
        expires_at: u64,
        version: u64,
        data: Bytes,
    ) -> Self {
        Self {
            meta,
            user_meta,
            expires_at,
            version,
            value: data,
        }
    }

    /// Decodes value from bytes
    #[inline]
    pub fn decode_bytes(src: Bytes) -> Self {
        let meta = src[META_OFFSET];
        let user_meta = src[USER_META_OFFSET];
        let (expires_at, sz) = binary_uvarint(&src[EXPIRATION_OFFSET..]);
        let value = src.slice(EXPIRATION_OFFSET + sz..);

        Self {
            meta,
            user_meta,
            expires_at,
            version: 0,
            value,
        }
    }

    /// Set the meta for the value
    #[inline]
    pub const fn set_meta(mut self, meta: u8) -> Self {
        self.meta = meta;
        self
    }

    /// Set the user meta for the value
    #[inline]
    pub const fn set_user_meta(mut self, user_meta: u8) -> Self {
        self.user_meta = user_meta;
        self
    }

    /// Set the expires_at for the value
    #[inline]
    pub const fn set_expires_at(mut self, expires_at: u64) -> Self {
        self.expires_at = expires_at;
        self
    }

    /// Set the version for the value
    #[inline]
    pub const fn set_version(mut self, version: u64) -> Self {
        self.version = version;
        self
    }

    /// Set the data for the value
    #[inline]
    pub fn set_data(mut self, value: Bytes) -> Self {
        self.value = value;
        self
    }

    /// Returns the version for this value
    #[inline]
    pub const fn get_version(&self) -> u64 {
        self.version
    }

    /// Returns the number of bytes contained in the value data.
    #[inline]
    pub fn len(&self) -> usize {
        self.value.len()
    }

    /// Returns true if the value data has a length of 0.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }
}

impl ValueExt for Value {
    #[inline]
    fn as_value_ref(&self) -> ValueRef {
        ValueRef {
            meta: self.meta,
            user_meta: self.user_meta,
            expires_at: self.expires_at,
            version: self.version,
            val: self.parse_value(),
        }
    }

    #[inline]
    fn parse_value(&self) -> &[u8] {
        self.value.as_ref()
    }

    #[inline]
    fn parse_value_to_bytes(&self) -> Bytes {
        self.value.clone()
    }

    #[inline]
    fn get_meta(&self) -> u8 {
        self.meta
    }

    #[inline]
    fn get_user_meta(&self) -> u8 {
        self.user_meta
    }

    #[inline]
    fn get_expires_at(&self) -> u64 {
        self.expires_at
    }

    impl_psfix_suites!(ValueExt::parse_value, u8, "u8");
}

fn size_variant(mut x: u64) -> usize {
    let mut n = 0;
    loop {
        n += 1;
        x >>= 7;
        if x == 0 {
            break;
        }
    }
    n
}

macro_rules! impl_from_for_value {
    ($($ty: ty), +$(,)?) => {
        $(
        impl From<$ty> for Value {
            fn from(val: $ty) -> Self {
                Self {
                    meta: 0,
                    user_meta: 0,
                    expires_at: 0,
                    value: Bytes::from(val),
                    version: 0,
                }
            }
        }
        )*
    };
}

impl_from_for_value! {
    String,
    &'static str,
    &'static [u8],
    Vec<u8>,
    Box<[u8]>,
    Bytes,
    BytesMut,
}

/// Extensions for `Value`
pub trait ValueExt {
    /// Returns a [`ValueRef`]
    ///
    /// [`ValueRef`]: struct.ValueRef.html
    #[inline]
    fn as_value_ref(&self) -> ValueRef {
        ValueRef {
            meta: self.get_meta(),
            user_meta: self.get_user_meta(),
            expires_at: self.get_expires_at(),
            version: 0,
            val: self.parse_value(),
        }
    }

    /// Returns the value data
    fn parse_value(&self) -> &[u8];

    /// Returns the value data (do shallow copy, except [`RawValuePointer`][`RawValuePointer`])
    ///
    /// [`RawValuePointer`]: struct.RawValuePointer.html
    fn parse_value_to_bytes(&self) -> Bytes;

    /// Get the meta of the value
    fn get_meta(&self) -> u8;

    /// Get the user meta
    fn get_user_meta(&self) -> u8;

    /// Returns the expiration time (unix timestamp) for this value
    fn get_expires_at(&self) -> u64;

    /// Returns the size of the Value when encoded
    #[inline]
    fn encoded_size(&self) -> u32 {
        let sz = self.parse_value().len() + 2; // meta, user meta.
        let enc = size_variant(self.get_expires_at());
        (sz + enc) as u32
    }

    /// Encode to a mutable slice. This function will copy the value.
    /// Use [`to_encoded`], if you want a shallow copy when encoded.
    ///
    /// # Panics
    /// This function panics if the remaining capacity of slice is less than encoded size.
    ///
    /// [`to_encoded`]: #method.to_encoded
    fn encode(&self, mut buf: &mut [u8]) {
        buf.put_u8(self.get_meta());
        buf.put_u8(self.get_user_meta());
        buf.put_slice(binary_uvarint_allocate(self.get_expires_at()).as_slice());
        buf.put_slice(self.parse_value());
    }

    /// Encode to a mutable buf. This function will copy the value.
    /// Use [`to_encoded`], if you want a shallow copy when encoded.
    ///
    /// # Panics
    /// This function panics if the remaining capacity of slice is less than encoded size.
    ///
    /// [`to_encoded`]: #method.to_encoded_to_buf
    fn encode_to_buf(&self, mut buf: impl BufMut) {
        buf.put_u8(self.get_meta());
        buf.put_u8(self.get_user_meta());
        buf.put_slice(binary_uvarint_allocate(self.get_expires_at()).as_slice());
        buf.put_slice(self.parse_value());
    }

    /// Encode to [`EncodedValue`].
    ///
    /// This function may be optimized by the underlying type to avoid actual copies.
    /// For example, [`Value`] implementation will do a shallow copy (ref-count increment)
    ///
    /// [`EncodedValue`]: struct.EncodedValue.html
    /// [`Value`]: struct.Value.html
    #[inline]
    fn to_encoded(&self) -> EncodedValue {
        let mut data = Vec::with_capacity(MAX_VALUE_INFO_SIZE);
        data.push(self.get_meta());
        data.push(self.get_user_meta());
        put_binary_uvariant_to_vec(data.as_mut(), self.get_expires_at());

        let expires_sz = data.len() - 2;
        let meta = Bytes::from(data);
        let val = self.parse_value_to_bytes();
        let enc_len = meta.len() + val.len();

        EncodedValue {
            data: meta.chain(val).copy_to_bytes(enc_len),
            expires_sz: expires_sz as u8,
        }
    }

    /// Decodes byte slice to value ref.
    #[inline]
    fn decode_value_ref(src: &[u8]) -> ValueRef {
        let meta = src[META_OFFSET];
        let user_meta = src[USER_META_OFFSET];
        let (expires_at, sz) = binary_uvarint(&src[EXPIRATION_OFFSET..]);
        ValueRef {
            meta,
            user_meta,
            expires_at,
            version: 0,
            val: &src[EXPIRATION_OFFSET + sz..],
        }
    }

    /// Decodes byte slice to value.
    #[inline]
    fn decode_value(src: &[u8]) -> Value {
        let meta = src[META_OFFSET];
        let user_meta = src[USER_META_OFFSET];
        let (expires_at, sz) = binary_uvarint(&src[EXPIRATION_OFFSET..]);
        let value = src[EXPIRATION_OFFSET + sz..].to_vec().into();

        Value {
            meta,
            user_meta,
            expires_at,
            version: 0,
            value,
        }
    }

    /// Decode bytes to value. (Shallow copy)
    #[inline]
    fn decode_bytes(src: Bytes) -> Value {
        let meta = src[META_OFFSET];
        let user_meta = src[USER_META_OFFSET];
        let (expires_at, sz) = binary_uvarint(&src[EXPIRATION_OFFSET..]);
        let value = src.slice(EXPIRATION_OFFSET + sz..);

        Value {
            meta,
            user_meta,
            expires_at,
            version: 0,
            value,
        }
    }

    impl_psfix_suites!(ValueExt::parse_value, u8, "u8");
}

/// ValueRef contains a `&'a Value`
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ValueRef<'a> {
    pub(crate) meta: u8,
    pub(crate) user_meta: u8,
    pub(crate) expires_at: u64,
    pub(crate) version: u64, // This field is not serialized. Only for internal usage.
    pub(crate) val: &'a [u8],
}

impl<'a> ValueRef<'a> {
    /// Returns a ValueRef from byte slice
    #[inline]
    pub const fn new(
        meta: u8,
        user_meta: u8,
        expires_at: u64,
        version: u64,
        data: &'a [u8],
    ) -> Self {
        Self {
            meta,
            user_meta,
            expires_at,
            version,
            val: data,
        }
    }

    /// Returns a ValueRef from [`RawKeyPointer`]
    ///
    /// # Safety
    /// The inner raw pointer of [`RawKeyPointer`] must be valid.
    ///
    /// [`RawKeyPointer`]: struct.RawKeyPointer.html
    #[inline]
    pub unsafe fn from_raw_value_pointer(rp: RawValuePointer) -> ValueRef<'a> {
        ValueRef {
            meta: rp.meta,
            user_meta: rp.user_meta,
            expires_at: rp.expires_at,
            version: rp.version,
            val: from_raw_parts(rp.ptr, rp.l as usize),
        }
    }

    /// Converts a slice of bytes to a string, including invalid characters.
    #[inline]
    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        String::from_utf8_lossy(self.val).to_string()
    }

    /// Converts a slice of bytes to a string, including invalid characters.
    #[inline]
    pub fn to_lossy_string(&self) -> Cow<'_, str> {
        String::from_utf8_lossy(self.val)
    }

    /// Copy the data to a new value
    #[inline]
    pub fn to_value(&self) -> Value {
        Value {
            meta: self.meta,
            user_meta: self.user_meta,
            expires_at: self.expires_at,
            version: self.version,
            value: Bytes::copy_from_slice(self.val),
        }
    }

    /// Get the value version
    #[inline]
    pub fn get_version(&self) -> u64 {
        self.version
    }

    /// Set the value version for this value ref
    #[inline]
    pub fn set_version(&mut self, version: u64) {
        self.version = version
    }
}

impl<'a> ValueExt for ValueRef<'a> {
    #[inline]
    fn as_value_ref(&self) -> ValueRef {
        *self
    }

    #[inline]
    fn parse_value(&self) -> &[u8] {
        self.val
    }

    #[inline]
    fn parse_value_to_bytes(&self) -> Bytes {
        Bytes::copy_from_slice(self.val)
    }

    #[inline]
    fn get_meta(&self) -> u8 {
        self.meta
    }

    #[inline]
    fn get_user_meta(&self) -> u8 {
        self.user_meta
    }

    #[inline]
    fn get_expires_at(&self) -> u64 {
        self.expires_at
    }
}

impl<const N: usize> ValueExt for [u8; N] {
    fn parse_value(&self) -> &[u8] {
        self
    }

    fn parse_value_to_bytes(&self) -> Bytes {
        Bytes::copy_from_slice(self)
    }

    fn get_meta(&self) -> u8 {
        self[META_OFFSET]
    }

    fn get_user_meta(&self) -> u8 {
        self[USER_META_OFFSET]
    }

    fn get_expires_at(&self) -> u64 {
        let (expires_at, _) = binary_uvarint(&self[EXPIRATION_OFFSET..]);
        expires_at
    }
}

impl<'a> ValueExt for &'a [u8] {
    fn parse_value(&self) -> &[u8] {
        self
    }

    fn parse_value_to_bytes(&self) -> Bytes {
        Bytes::copy_from_slice(self)
    }

    fn get_meta(&self) -> u8 {
        self[META_OFFSET]
    }

    fn get_user_meta(&self) -> u8 {
        self[USER_META_OFFSET]
    }

    fn get_expires_at(&self) -> u64 {
        let (expires_at, _) = binary_uvarint(&self[EXPIRATION_OFFSET..]);
        expires_at
    }
}

impl ValueExt for Box<[u8]> {
    fn parse_value(&self) -> &[u8] {
        self
    }

    fn parse_value_to_bytes(&self) -> Bytes {
        Bytes::copy_from_slice(self)
    }

    fn get_meta(&self) -> u8 {
        self[META_OFFSET]
    }

    fn get_user_meta(&self) -> u8 {
        self[USER_META_OFFSET]
    }

    fn get_expires_at(&self) -> u64 {
        let (expires_at, _) = binary_uvarint(&self[EXPIRATION_OFFSET..]);
        expires_at
    }
}

impl ValueExt for Arc<[u8]> {
    fn parse_value(&self) -> &[u8] {
        self
    }

    fn parse_value_to_bytes(&self) -> Bytes {
        Bytes::copy_from_slice(self)
    }

    fn get_meta(&self) -> u8 {
        self[META_OFFSET]
    }

    fn get_user_meta(&self) -> u8 {
        self[USER_META_OFFSET]
    }

    fn get_expires_at(&self) -> u64 {
        let (expires_at, _) = binary_uvarint(&self[EXPIRATION_OFFSET..]);
        expires_at
    }
}

impl ValueExt for Rc<[u8]> {
    fn parse_value(&self) -> &[u8] {
        self
    }

    fn parse_value_to_bytes(&self) -> Bytes {
        Bytes::copy_from_slice(self)
    }

    fn get_meta(&self) -> u8 {
        self[META_OFFSET]
    }

    fn get_user_meta(&self) -> u8 {
        self[USER_META_OFFSET]
    }

    fn get_expires_at(&self) -> u64 {
        let (expires_at, _) = binary_uvarint(&self[EXPIRATION_OFFSET..]);
        expires_at
    }
}
