use crate::{Value, ValueExt, ValueRef};
use bytes::{Bytes, BytesMut};
use core::ops::{Deref, DerefMut};

/// ValueMut represents the value info that can be associated with a key, but also the internal
/// Meta field. The data in the ValueMut is mutable.
///
/// # Design for ValueMut
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
pub struct ValueMut {
    pub(crate) meta: u8,
    pub(crate) user_meta: u8,
    pub(crate) expires_at: u64,
    pub(crate) version: u64, // This field is not serialized. Only for internal usage.
    pub(crate) value: BytesMut,
}

impl ValueMut {
    /// Freeze to Value.
    #[inline]
    pub fn freeze(self) -> Value {
        Value {
            meta: self.meta,
            user_meta: self.user_meta,
            expires_at: self.expires_at,
            version: self.version,
            value: self.value.freeze(),
        }
    }
}

impl Deref for ValueMut {
    type Target = BytesMut;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl DerefMut for ValueMut {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

impl ValueExt for ValueMut {
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

    /// Unlike `Value` do shallow copy, the implementation for `parse_value_to_bytes`
    /// in `ValueMut` will full copy.
    #[inline]
    fn parse_value_to_bytes(&self) -> Bytes {
        Bytes::copy_from_slice(self.value.as_ref())
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

impl ValueMutExt for ValueMut {
    #[inline]
    fn parse_value_mut(&mut self) -> &mut [u8] {
        self.value.as_mut()
    }

    #[inline]
    fn set_meta(&mut self, meta: u8) {
        self.meta = meta
    }

    #[inline]
    fn set_user_meta(&mut self, user_meta: u8) {
        self.user_meta = user_meta
    }

    #[inline]
    fn set_expires_at(&mut self, expires_at: u64) {
        self.expires_at = expires_at
    }
}

/// Extensions for `ValueMut`
pub trait ValueMutExt {
    /// Returns the mutable data slice store in ValueMut
    fn parse_value_mut(&mut self) -> &mut [u8];

    /// Set the meta of the value
    fn set_meta(&mut self, meta: u8);

    /// Set the user meta
    fn set_user_meta(&mut self, user_meta: u8);

    /// Set the expiration time (unix timestamp) for this value
    fn set_expires_at(&mut self, expires_at: u64);
}
