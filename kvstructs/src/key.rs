use crate::key_mut::KeyMut;
use crate::raw_key_pointer::RawKeyPointer;
use crate::{u64_big_endian, TIMESTAMP_SIZE};
use alloc::borrow::Cow;
use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use bytes::{Buf, Bytes, BytesMut};
use core::cmp::Ordering;
use core::hash::{Hash, Hasher};
use core::ops::{Deref, DerefMut};
use core::slice::from_raw_parts;
#[cfg(feature = "std")]
use std::time::{SystemTime, UNIX_EPOCH};

/// A general Key for key-value storage, the underlying is u8 slice.
#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct Key {
    data: Bytes,
}

impl Default for Key {
    fn default() -> Self {
        Self::new()
    }
}

impl Key {
    /// Returns a empty key
    #[inline]
    pub const fn new() -> Self {
        Self { data: Bytes::new() }
    }

    /// Returns a Key with data and timestamp.
    #[inline]
    pub fn from_with_timestamp(data: Vec<u8>, ts: u64) -> Self {
        Self::from(data).with_timestamp(ts)
    }

    /// Returns a Key with data and system time as timestamp.
    #[cfg(feature = "std")]
    #[inline]
    pub fn from_with_system_time(data: Vec<u8>, st: SystemTime) -> Self {
        Self::from(data).with_system_time(st)
    }

    /// Returns a Key with data and the current time as timestamp
    #[cfg(feature = "std")]
    #[inline]
    pub fn from_with_now(data: Vec<u8>) -> Self {
        Self::from(data).with_now()
    }

    /// Returns a Key by copying the slice data.
    #[inline]
    pub fn copy_from_slice(data: &[u8]) -> Self {
        Bytes::copy_from_slice(data).into()
    }

    /// Generates a new key by appending timestamp to key.
    #[inline]
    pub fn with_timestamp(self, ts: u64) -> Self {
        let len = self.data.len() + TIMESTAMP_SIZE;
        let ts = Bytes::from(Box::from((u64::MAX - ts).to_be_bytes()));
        self.data.chain(ts).copy_to_bytes(len).into()
    }

    /// Generates a new key by appending the given UNIX system time to key.
    #[inline]
    #[cfg(feature = "std")]
    pub fn with_system_time(self, st: SystemTime) -> Self {
        let len = self.data.len() + TIMESTAMP_SIZE;
        let ts = Bytes::from(Box::from(
            st.duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                .to_be_bytes(),
        ));
        self.data.chain(ts).copy_to_bytes(len).into()
    }

    /// Generates a new key by appending the current UNIX system time to key.
    #[inline]
    #[cfg(feature = "std")]
    pub fn with_now(self) -> Self {
        let len = self.data.len() + TIMESTAMP_SIZE;
        let ts = Bytes::from(Box::from(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                .to_be_bytes(),
        ));
        self.data.chain(ts).copy_to_bytes(len).into()
    }

    /// Returns a new Key without timestamp.
    #[inline]
    pub fn parse_new_key(&self) -> Self {
        let sz = self.len();
        match sz.checked_sub(TIMESTAMP_SIZE) {
            None => Self {
                data: self.data.clone(),
            },
            Some(sz) => Self {
                data: self.data.slice(..sz),
            },
        }
    }

    /// Returns the number of bytes contained in this Key.
    #[inline]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns true if the Key has a length of 0.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Returns the underlying bytes
    #[inline]
    pub fn as_slice(&self) -> &[u8] {
        self.data.as_ref()
    }

    /// Remove the timestamp(if exists) from the key
    pub fn truncate_timestamp(&mut self) {
        if let Some(sz) = self.data.len().checked_sub(TIMESTAMP_SIZE) {
            self.data.truncate(sz)
        }
    }
}

impl From<Key> for Bytes {
    #[inline]
    fn from(key: Key) -> Self {
        key.data
    }
}

impl<'a, K: KeyExt> PartialEq<K> for KeyRef<'a> {
    fn eq(&self, other: &K) -> bool {
        same_key_in(self.as_bytes(), other.as_bytes())
    }
}

impl<K: KeyExt> PartialEq<K> for Key {
    fn eq(&self, other: &K) -> bool {
        same_key_in(self.as_bytes(), other.as_bytes())
    }
}

impl<K: KeyExt> PartialOrd<K> for Key {
    fn partial_cmp(&self, other: &K) -> Option<Ordering> {
        Some(compare_key_in(self.as_bytes(), other.as_bytes()))
    }
}

impl<'a, K: KeyExt> PartialOrd<K> for KeyRef<'a> {
    fn partial_cmp(&self, other: &K) -> Option<Ordering> {
        Some(compare_key_in(self.as_bytes(), other.as_bytes()))
    }
}

impl Eq for Key {}

impl Hash for Key {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.data.hash(state)
    }
}

impl Ord for Key {
    /// Checks the key without timestamp and checks the timestamp if keyNoTs
    /// is same.
    /// a<timestamp> would be sorted higher than aa<timestamp> if we use bytes.compare
    /// All keys should have timestamp.
    fn cmp(&self, other: &Self) -> Ordering {
        compare_key_in(self.data.as_ref(), other.data.as_ref())
    }
}

#[inline(always)]
pub(crate) fn compare_key_in(me: &[u8], other: &[u8]) -> Ordering {
    let sb = me.len().saturating_sub(TIMESTAMP_SIZE);
    let ob = other.len().saturating_sub(TIMESTAMP_SIZE);
    let (s_key_part, s_ts_part) = me.split_at(sb);
    let (o_key_part, o_ts_part) = other.split_at(ob);

    match s_key_part.cmp(o_key_part) {
        Ordering::Less => Ordering::Less,
        Ordering::Equal => s_ts_part.cmp(o_ts_part),
        Ordering::Greater => Ordering::Greater,
    }
}

/// Checks the key without timestamp and checks the timestamp if keyNoTs
/// is same.
/// a<timestamp> would be sorted higher than aa<timestamp> if we use bytes.compare
/// All keys should have timestamp.
#[inline(always)]
pub fn compare_key(a: impl KeyExt, b: impl KeyExt) -> Ordering {
    let me = a.as_bytes();
    let other = b.as_bytes();
    compare_key_in(me, other)
}

#[inline(always)]
pub(crate) fn same_key_in(me: &[u8], other: &[u8]) -> bool {
    let sl = me.len();
    let ol = other.len();
    if sl != ol {
        false
    } else {
        let s = match sl.checked_sub(TIMESTAMP_SIZE) {
            None => me,
            Some(sz) => me[..sz].as_ref(),
        };
        let o = match ol.checked_sub(TIMESTAMP_SIZE) {
            None => me,
            Some(sz) => other[..sz].as_ref(),
        };
        s.eq(o)
    }
}

/// Checks for key equality ignoring the version timestamp.
#[inline(always)]
pub fn same_key(a: impl KeyExt, b: impl KeyExt) -> bool {
    let me = a.as_bytes();
    let other = b.as_bytes();
    same_key_in(me, other)
}

impl<const N: usize> From<[u8; N]> for Key {
    fn from(data: [u8; N]) -> Self {
        Self {
            data: Bytes::from(data.to_vec()),
        }
    }
}

macro_rules! impl_from_for_key {
    ($($ty: ty), +$(,)?) => {
        $(
        impl From<$ty> for Key {
            fn from(val: $ty) -> Self {
                Self {
                    data: Bytes::from(val),
                }
            }
        }
        )*
    };
}

impl_from_for_key! {
    String,
    &'static str,
    Vec<u8>,
    Box<[u8]>,
}

impl Deref for Key {
    type Target = Bytes;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl DerefMut for Key {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl From<Bytes> for Key {
    fn from(data: Bytes) -> Self {
        Self { data }
    }
}

impl From<BytesMut> for Key {
    fn from(data: BytesMut) -> Self {
        Self {
            data: data.freeze(),
        }
    }
}

impl From<&[u8]> for Key {
    fn from(data: &[u8]) -> Self {
        Key::copy_from_slice(data)
    }
}

impl AsRef<[u8]> for Key {
    fn as_ref(&self) -> &[u8] {
        self.data.as_ref()
    }
}

/// KeyRef can only contains a underlying u8 slice of Key
#[derive(Debug, Copy, Clone)]
#[repr(transparent)]
pub struct KeyRef<'a> {
    data: &'a [u8],
}

impl<'a> Eq for KeyRef<'a> {}

impl<'a> Ord for KeyRef<'a> {
    /// Checks the key without timestamp and checks the timestamp if keyNoTs
    /// is same.
    /// a<timestamp> would be sorted higher than aa<timestamp> if we use bytes.compare
    /// All keys should have timestamp.
    fn cmp(&self, other: &Self) -> Ordering {
        compare_key(self, other)
    }
}

impl<'a> From<&'a [u8]> for KeyRef<'a> {
    fn from(data: &'a [u8]) -> Self {
        Self { data }
    }
}

impl<'a> KeyRef<'a> {
    /// Returns a KeyRef from byte slice
    #[inline]
    pub const fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

    /// Returns a KeyRef from [`RawKeyPointer`]
    ///
    /// # Safety
    /// The inner raw pointer of [`RawKeyPointer`] must be valid.
    ///
    /// [`RawKeyPointer`]: struct.RawKeyPointer.html
    #[inline]
    pub unsafe fn from_raw_key_pointer(rp: RawKeyPointer) -> Self {
        Self {
            data: from_raw_parts(rp.as_ptr(), rp.len()),
        }
    }

    /// Returns a KeyRef from raw pointer and length
    ///
    /// # Safety
    /// The raw pointer must be valid.
    #[inline]
    pub unsafe fn from_raw_pointer(ptr: *const u8, len: usize) -> Self {
        Self {
            data: from_raw_parts(ptr, len),
        }
    }

    /// Copy KeyRef to a new Key.
    #[inline]
    pub fn to_key(&self) -> Key {
        Key::copy_from_slice(self.data)
    }

    /// Returns the number of bytes contained in this Key.
    #[inline]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns true if the Key has a length of 0.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Returns the underlying bytes
    #[inline]
    pub fn as_slice(&self) -> &[u8] {
        self.data
    }
}

impl KeyExt for &'_ KeyRef<'_> {
    #[inline]
    fn as_bytes(&self) -> &[u8] {
        self.data
    }
}

impl KeyExt for &'_ mut KeyRef<'_> {
    #[inline]
    fn as_bytes(&self) -> &[u8] {
        self.data
    }
}

impl KeyExt for KeyRef<'_> {
    #[inline]
    fn as_bytes(&self) -> &[u8] {
        self.data
    }
}

impl Hash for KeyRef<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.data.hash(state)
    }
}

/// Extensions for Key
pub trait KeyExt {
    /// Returns raw pointer of the underlying byte slice
    #[inline]
    fn as_ptr(&self) -> *const u8 {
        self.as_bytes().as_ptr()
    }

    /// Returns a KeyRef.
    #[inline]
    fn as_key_ref(&self) -> KeyRef {
        KeyRef {
            data: self.as_bytes(),
        }
    }

    /// Returns the underlying slice of key (with timestamp data).
    fn as_bytes(&self) -> &[u8];

    /// Parses the actual key from the key bytes.
    #[inline]
    fn parse_key(&self) -> &[u8] {
        let data = self.as_bytes();
        let sz = data.len();
        match sz.checked_sub(TIMESTAMP_SIZE) {
            None => data,
            Some(sz) => data[..sz].as_ref(),
        }
    }

    /// Parses the timestamp from the key bytes.
    ///
    /// # Panics
    /// If the length of key less than 8.
    #[inline]
    fn parse_timestamp(&self) -> u64 {
        let data = self.as_bytes();
        let data_len = data.len();
        if data_len <= TIMESTAMP_SIZE {
            0
        } else {
            u64::MAX - u64_big_endian(&data[data_len - TIMESTAMP_SIZE..])
        }
    }

    /// Checks for key equality ignoring the version timestamp.
    #[inline]
    fn same_key(&self, other: impl KeyExt) -> bool {
        let me = self.as_bytes();
        let other = other.as_bytes();
        same_key_in(me, other)
    }

    /// Checks the key without timestamp and checks the timestamp if keyNoTs
    /// is same.
    /// a<timestamp> would be sorted higher than aa<timestamp> if we use bytes.compare
    /// All keys should have timestamp.
    #[inline]
    fn compare_key(&self, other: impl KeyExt) -> Ordering {
        let me = self.as_bytes();
        let other = other.as_bytes();
        compare_key_in(me, other)
    }

    impl_psfix_suites!(KeyExt::parse_key, u8, "u8");
}

macro_rules! impl_partial_eq_ord {
    ($($ty:ty), +$(,)?) => {
        $(
        impl PartialEq<Key> for $ty {
            fn eq(&self, other: &Key) -> bool {
                other.same_key(self)
            }
        }

        impl<'a> PartialEq<KeyRef<'a>> for $ty {
            fn eq(&self, other: &KeyRef<'a>) -> bool {
                other.same_key(self)
            }
        }

        // impl<'a> PartialEq<$ty> for KeyRef<'a> {
        //     fn eq(&self, other: &$ty) -> bool {
        //         self.same_key(other)
        //     }
        // }

        impl PartialOrd<Key> for $ty {
            fn partial_cmp(&self, other: &Key) -> Option<Ordering> {
                Some(compare_key(other, self))
            }
        }

        impl<'a> PartialOrd<KeyRef<'a>> for $ty {
            fn partial_cmp(&self, other: &KeyRef<'a>) -> Option<Ordering> {
                Some(compare_key(other, self))
            }
        }
        )*
    };
}

macro_rules! impl_key_ext {
    ($($ty:tt::$conv:tt), +$(,)?) => {
        $(
        impl KeyExt for $ty {
            #[inline]
            fn as_bytes(&self) -> &[u8] {
                $ty::$conv(self)
            }
        }

        impl<'a> KeyExt for &'a $ty {
            #[inline]
            fn as_bytes(&self) -> &[u8] {
                $ty::$conv(self)
            }
        }

        impl<'a> KeyExt for &'a mut $ty {
            #[inline]
            fn as_bytes(&self) -> &[u8] {
                $ty::$conv(self)
            }
        }
        )*
    };
}

type VecBytes = Vec<u8>;
type U8Bytes = [u8];
type BoxBytes = Box<[u8]>;

impl_partial_eq_ord! {
    Bytes,
    BytesMut,
    BoxBytes,
    KeyMut,
    U8Bytes,
    VecBytes,
    str,
    String,
}

impl_key_ext! {
    Bytes::as_ref,
    BytesMut::as_ref,
    BoxBytes::as_ref,
    Key::as_ref,
    U8Bytes::as_ref,
    VecBytes::as_slice,
    str::as_bytes,
    String::as_bytes,
}

impl<const N: usize> PartialEq<Key> for [u8; N] {
    fn eq(&self, other: &Key) -> bool {
        other.same_key(self)
    }
}

// impl<const N: usize> PartialEq<[u8; N]> for Key {
//     fn eq(&self, other: &[u8; N]) -> bool {
//         self.same_key(other)
//     }
// }

impl<const N: usize> KeyExt for [u8; N] {
    #[inline]
    fn as_bytes(&self) -> &[u8] {
        self
    }
}

impl<'a, const N: usize> KeyExt for &'a [u8; N] {
    #[inline]
    fn as_bytes(&self) -> &[u8] {
        self.as_slice()
    }
}

impl<'a, const N: usize> KeyExt for &'a mut [u8; N] {
    #[inline]
    fn as_bytes(&self) -> &[u8] {
        self.as_slice()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use alloc::vec;

    #[test]
    fn key_integration_test() {
        // test key_with_ts
        let key = vec![0, 1, 2, 3, 4, 5, 6, 7];
        let nk = Key::from(key.clone()).with_timestamp(10);
        assert_eq!(
            vec![0, 1, 2, 3, 4, 5, 6, 7, 255, 255, 255, 255, 255, 255, 255, 245],
            nk
        );

        // test parse_ts
        assert_eq!(nk.parse_timestamp(), 10);

        // test parse_key
        let nk2 = Key::from(key).with_timestamp(1000);
        assert_eq!(nk.parse_key(), nk2.parse_key());

        // test cmp
        assert!(nk.cmp(&nk2).is_gt());

        // test same key
        assert_eq!(nk, nk2);
    }
}
