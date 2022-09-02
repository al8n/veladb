use crate::{binary_uvarint, ValueExt, ValueRef, EXPIRATION_OFFSET};
use core::cmp::Ordering;
use core::hash::{Hash, Hasher};
use core::ops::Deref;
use core::slice::from_raw_parts;

/// RawValuePointer contains a raw pointer of the data of [`Value`]
/// This struct is unsafe, because it does not promise the raw pointer always valid.
///
/// [`Value`]: struct.Value.html
#[derive(Debug, Copy, Clone)]
pub struct RawValuePointer {
    pub(crate) meta: u8,
    pub(crate) user_meta: u8,
    pub(crate) version: u64, // This field is not serialized. Only for internal usage.
    pub(crate) ptr: *const u8,
    pub(crate) l: u32,
    pub(crate) expires_at: u64,
}

impl RawValuePointer {
    /// Returns a RawValuePointer
    ///
    /// # Safety
    /// The inner raw pointer must be valid.
    pub unsafe fn new(ptr: *const u8, len: u32) -> Self {
        let buf = from_raw_parts(ptr, len as usize);
        let (expires_at, sz) = binary_uvarint(&buf[EXPIRATION_OFFSET..]);
        let val_len = len as usize - (EXPIRATION_OFFSET + sz);

        Self {
            meta: buf[0],
            user_meta: buf[1],
            version: 0,
            ptr: ptr.add(EXPIRATION_OFFSET + sz),
            l: val_len as u32,
            expires_at,
        }
    }

    /// Returns a [`ValueRef`] according to the inner raw value pointer
    ///
    /// # Safety
    /// The inner raw pointer must be valid.
    #[inline(always)]
    pub unsafe fn as_value_ref(&self) -> ValueRef<'_> {
        ValueRef {
            meta: self.meta,
            user_meta: self.user_meta,
            expires_at: self.expires_at,
            version: self.version,
            val: from_raw_parts(self.ptr, self.l as usize),
        }
    }

    /// Set the version for this value
    #[inline]
    pub fn set_version(&mut self, version: u64) {
        self.version = version;
    }

    /// Get the version for this value
    #[inline]
    pub fn get_version(&self) -> u64 {
        self.version
    }
}

impl Deref for RawValuePointer {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        unsafe { from_raw_parts(self.ptr, self.l as usize) }
    }
}

impl Hash for RawValuePointer {
    fn hash<H: Hasher>(&self, state: &mut H) {
        unsafe { self.as_value_ref().hash(state) }
    }
}

impl PartialEq<RawValuePointer> for RawValuePointer {
    fn eq(&self, other: &RawValuePointer) -> bool {
        unsafe { self.as_value_ref().eq(&other.as_value_ref()) }
    }
}

impl Eq for RawValuePointer {}

impl ValueExt for RawValuePointer {
    fn parse_value(&self) -> &[u8] {
        self
    }

    fn parse_value_to_bytes(&self) -> bytes::Bytes {
        bytes::Bytes::copy_from_slice(self)
    }

    fn get_meta(&self) -> u8 {
        self.meta
    }

    fn get_user_meta(&self) -> u8 {
        self.user_meta
    }

    fn get_expires_at(&self) -> u64 {
        self.expires_at
    }
}

impl PartialOrd<RawValuePointer> for RawValuePointer {
    fn partial_cmp(&self, other: &RawValuePointer) -> Option<Ordering> {
        unsafe { self.as_value_ref().partial_cmp(&other.as_value_ref()) }
    }
}

impl Ord for RawValuePointer {
    fn cmp(&self, other: &Self) -> Ordering {
        unsafe { self.as_value_ref().cmp(&other.as_value_ref()) }
    }
}
