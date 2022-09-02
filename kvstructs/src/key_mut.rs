use crate::{compare_key_in, same_key_in, Key, KeyExt, TIMESTAMP_SIZE};
use bytes::{BufMut, BytesMut};
use core::cmp::Ordering;
use core::hash::{Hash, Hasher};
use core::ops::{Deref, DerefMut};

/// A general mutable Key for key-value storage, the underlying is u8 slice.
#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct KeyMut {
    data: BytesMut,
}

impl Default for KeyMut {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for KeyMut {
    type Target = BytesMut;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl DerefMut for KeyMut {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl AsRef<[u8]> for KeyMut {
    fn as_ref(&self) -> &[u8] {
        self.data.as_ref()
    }
}

impl AsMut<[u8]> for KeyMut {
    fn as_mut(&mut self) -> &mut [u8] {
        self.data.as_mut()
    }
}

impl PartialEq<Self> for KeyMut {
    fn eq(&self, other: &Self) -> bool {
        same_key_in(self.data.as_ref(), other.data.as_ref())
    }
}

impl Eq for KeyMut {}

impl Hash for KeyMut {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.data.hash(state)
    }
}

impl PartialOrd<Self> for KeyMut {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for KeyMut {
    /// Checks the key without timestamp and checks the timestamp if keyNoTs
    /// is same.
    /// a<timestamp> would be sorted higher than aa<timestamp> if we use bytes.compare
    /// All keys should have timestamp.
    fn cmp(&self, other: &Self) -> Ordering {
        compare_key_in(self.data.as_ref(), other.data.as_ref())
    }
}

impl KeyMut {
    /// Creates a new `KeyMut` with default capacity.
    ///
    /// Resulting object has length 0 and unspecified capacity.
    /// This function does not allocate.
    pub fn new() -> Self {
        Self {
            data: BytesMut::new(),
        }
    }

    /// Creates a new `KeyMut` with the specified capacity.
    ///
    /// The returned `KeyMut` will be able to hold at least `capacity` bytes
    /// without reallocating.
    ///
    /// It is important to note that this function does not specify the length
    /// of the returned `KeyMut`, but only the capacity.
    ///
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            data: BytesMut::with_capacity(cap),
        }
    }

    /// Generates a new key by appending timestamp to key.
    #[inline]
    pub fn with_timestamp(mut self, ts: u64) -> Self {
        self.data.put_u64(ts);
        self
    }

    /// Converts self into an immutable Key.
    /// The conversion is zero cost and is used to indicate that
    /// the slice referenced by the handle will no longer be mutated.
    /// Once the conversion is done, the handle can be cloned and shared across threads
    pub fn freeze(self) -> Key {
        Key::from(self.data.freeze())
    }
}

impl<'a> KeyExt for &'a KeyMut {
    fn as_bytes(&self) -> &[u8] {
        self.data.as_ref()
    }
}

impl<'a> KeyExt for &'a mut KeyMut {
    fn as_bytes(&self) -> &[u8] {
        self.data.as_ref()
    }
}

impl KeyExt for KeyMut {
    fn as_bytes(&self) -> &[u8] {
        self.data.as_ref()
    }
}

/// Extensions for `KeyMut`
pub trait KeyMutExt {
    /// Returns the mutable underlying slice of key (with timestamp data).
    fn as_bytes_mut(&mut self) -> &mut [u8];

    /// Returns raw mutable pointer of the underlying byte slice
    #[inline]
    fn as_mut_ptr(&mut self) -> *mut u8 {
        self.as_bytes_mut().as_mut_ptr()
    }

    /// Returns the mutable data slice store in ValueMut
    fn parse_key_mut(&mut self) -> &mut [u8];

    /// Set the timestamp for key
    fn set_timestamp(&mut self, ts: u64);
}

impl KeyMutExt for KeyMut {
    #[inline]
    fn as_bytes_mut(&mut self) -> &mut [u8] {
        self.data.as_mut()
    }

    #[inline]
    fn parse_key_mut(&mut self) -> &mut [u8] {
        self.data.as_mut()
    }

    #[inline]
    fn set_timestamp(&mut self, ts: u64) {
        let sz = self.len();
        match sz.checked_sub(TIMESTAMP_SIZE) {
            None => self.data.put_u64(ts),
            Some(sz) => self.data[sz..].copy_from_slice(ts.to_be_bytes().as_slice()),
        }
    }
}

impl<'a> KeyMutExt for &'a mut KeyMut {
    #[inline]
    fn as_bytes_mut(&mut self) -> &mut [u8] {
        self.data.as_mut()
    }

    #[inline]
    fn parse_key_mut(&mut self) -> &mut [u8] {
        self.data.as_mut()
    }

    #[inline]
    fn set_timestamp(&mut self, ts: u64) {
        let sz = self.len();
        match sz.checked_sub(TIMESTAMP_SIZE) {
            None => self.data.put_u64(ts),
            Some(sz) => self.data[sz..].copy_from_slice(ts.to_be_bytes().as_slice()),
        }
    }
}
