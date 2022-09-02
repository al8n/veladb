use crate::raw_key_pointer::RawKeyPointer;
use crate::raw_value_pointer::RawValuePointer;
use crate::{EncodedValue, KeyRef, ValueExt, ValueRef};

/// RawEntryPointer contains a raw pointer of the data slice of [`Key`]
/// and a raw pointer of the data slice of [`Value`].
/// This struct is unsafe, because it does not promise the raw pointer always valid.
///
/// [`Key`]: struct.Key.html
/// [`Value`]: struct.Value.html
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct RawEntryPointer {
    key: RawKeyPointer,
    val: RawValuePointer,

    offset: u32,
    // Fields maintained internally.
    /// length of the header
    h_len: usize,
    val_threshold: u64,
}

impl RawEntryPointer {
    /// Returns a new RawEntryPointer
    ///
    /// # Safety
    /// The inner raw pointers must be valid.
    #[inline]
    pub const unsafe fn new(
        key: RawKeyPointer,
        val: RawValuePointer,
        offset: u32,
        h_len: usize,
        val_threshold: u64,
    ) -> Self {
        Self {
            key,
            val,
            offset,
            h_len,
            val_threshold,
        }
    }

    /// Get the KeyRef
    ///
    /// # Safety
    /// The inner raw key pointer must be valid.
    #[inline]
    pub unsafe fn key(&self) -> KeyRef {
        self.key.as_key_ref()
    }

    /// Get the ValueRef
    ///
    /// # Safety
    /// The inner raw value pointer must be valid.
    #[inline]
    pub unsafe fn value(&self) -> ValueRef {
        self.val.as_value_ref()
    }

    /// Get the offset of the entry
    #[inline]
    pub fn get_offset(&self) -> u32 {
        self.offset
    }

    /// Get the header length of the entry
    #[inline]
    pub fn get_header_len(&self) -> usize {
        self.h_len
    }

    /// Get the value threshold of the entry
    #[inline]
    pub fn get_value_threshold(&self) -> u64 {
        self.val_threshold
    }

    /// Returns the length of key
    ///
    /// # Safety
    /// The inner raw key pointer must be valid.
    #[inline]
    pub unsafe fn key_len(&self) -> usize {
        self.key.len()
    }

    /// Returns whether the key is empty.
    ///
    /// # Safety
    /// The inner raw key pointer must be valid.
    #[inline]
    pub unsafe fn is_key_empty(&self) -> bool {
        self.key.is_empty()
    }

    /// Returns the length of value
    ///
    /// # Safety
    /// The inner raw value pointer must be valid.
    #[inline]
    pub unsafe fn value_len(&self) -> usize {
        self.val.len()
    }

    /// Returns whether the value is empty.
    ///
    /// # Safety
    /// The inner raw value pointer must be valid.
    #[inline]
    pub unsafe fn is_value_empty(&self) -> bool {
        self.val.is_empty()
    }

    /// Get the encoded value.
    ///
    /// # Safety
    /// The inner raw value pointer must be valid.
    #[inline]
    pub unsafe fn encoded_value(&self) -> EncodedValue {
        self.value().to_encoded()
    }
}
