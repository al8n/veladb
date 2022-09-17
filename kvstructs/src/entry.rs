use crate::OP;
use crate::{EncodedValue, Key, Value, ValueExt};

/// Entry provides Key, Value, UserMeta and ExpiresAt. This struct can be used by
/// the user to set data.
#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct Entry {
    key: Key,
    val: Value,
    offset: u32,
    // Fields maintained internally.
    /// length of the header
    h_len: usize,
    val_threshold: u64,
}

impl Entry {
    /// Returns a new empty Entry.
    #[inline]
    pub const fn new() -> Self {
        Self {
            key: Key::new(),
            val: Value::new(),
            offset: 0,
            h_len: 0,
            val_threshold: 0,
        }
    }

    /// Creates a new entry with key and value passed in args. This newly created entry can be
    /// set in a transaction by calling txn.SetEntry(). All other properties of Entry can be set by
    /// calling with_meta, with_discard, with_ttl methods on it.
    /// This function uses key and value reference, hence users must
    /// not modify key and value until the end of transaction.
    #[inline]
    pub fn new_from_kv(key: Key, val: Value) -> Self {
        Self {
            key,
            val,
            offset: 0,
            h_len: 0,
            val_threshold: 0,
        }
    }

    /// Get the key
    #[inline]
    pub const fn get_key(&self) -> &Key {
        &self.key
    }

    /// Get the value
    #[inline]
    pub const fn get_value(&self) -> &Value {
        &self.val
    }

    /// Get the length of the header
    #[inline]
    pub const fn get_header_len(&self) -> usize {
        self.h_len
    }

    /// Get the header
    #[inline]
    pub fn get_header(&self) -> crate::header::Header {
        crate::header::Header {
            k_len: self.key.len() as u32,
            v_len: self.val.len() as u32,
            expires_at: self.val.get_expires_at(),
            meta: self.val.get_meta(),
            user_meta: self.val.get_user_meta(),
        }
    }

    /// Get the value threshold
    #[inline]
    pub const fn get_value_threshold(&self) -> u64 {
        self.val_threshold
    }

    /// Set the length of the header
    #[inline]
    pub fn set_header_len(&mut self, hlen: usize) {
        self.h_len = hlen
    }

    /// Adds meta data to Entry e. This byte is stored alongside the key
    /// and can be used as an aid to interpret the value or store other contextual
    /// bits corresponding to the key-value pair of entry.
    #[inline]
    pub fn set_meta(&mut self, meta: u8) {
        self.val.meta = meta;
    }

    /// Adds time to live duration to Entry e. Entry stored with a TTL would automatically expire
    /// after the time has elapsed, and will be eligible for garbage collection.
    #[inline]
    pub fn set_ttl(&mut self, dur: u64) {
        self.val.expires_at = dur;
    }

    /// Set the offset
    #[inline]
    pub fn set_offset(&mut self, offset: u32) {
        self.offset = offset;
    }

    /// Returns the offset
    #[inline]
    pub const fn get_offset(&self) -> u32 {
        self.offset
    }

    /// Adds a marker to Entry e. This means all the previous versions of the key (of the
    /// Entry) will be eligible for garbage collection.
    /// This method is only useful if you have set a higher limit for options.NumVersionsToKeep. The
    /// default setting is 1, in which case, this function doesn't add any more benefit. If however, you
    /// have a higher setting for NumVersionsToKeep (in Dgraph, we set it to infinity), you can use this
    /// method to indicate that all the older versions can be discarded and removed during compactions.
    #[inline]
    pub fn mark_discard(&mut self) {
        self.val.meta = OP::BIT_DISCARD_EARLIER_VERSIONS.bits();
    }

    /// mark_merge sets merge bit in entry's metadata. This
    /// function is called by MergeOperator's Add method.
    #[inline]
    pub fn mark_merge(&mut self) {
        self.val.meta = OP::BIT_MERGE_ENTRY.bits();
    }

    /// Returns the length of key
    #[inline]
    pub fn key_len(&self) -> usize {
        self.key.len()
    }

    /// Returns whether the key is empty.
    #[inline]
    pub fn is_key_empty(&self) -> bool {
        self.key.is_empty()
    }

    /// Returns the length of value
    #[inline]
    pub fn value_len(&self) -> usize {
        self.val.len()
    }

    /// Returns whether the value is empty.
    #[inline]
    pub fn is_value_empty(&self) -> bool {
        self.val.is_empty()
    }

    /// estimate the entry size and set the threshold
    #[inline]
    pub fn estimate_size_and_set_threshold(&mut self, threshold: u64) -> u64 {
        if self.val_threshold == 0 {
            self.val_threshold = threshold;
        }

        let klen = self.key.len() as u64;
        let vlen = self.val.len() as u64;
        if vlen < self.val_threshold {
            return klen + vlen + 2; // meta. user meta
        }

        klen + 12 + 2 // 12 for value pointer, 2 for meta
    }

    /// skip the value log and set the threshold
    #[inline]
    pub fn skip_vlog_and_set_threshold(&mut self, threshold: u64) -> bool {
        if self.val_threshold == 0 {
            self.val_threshold = threshold;
        }

        (self.val.len() as u64) < self.val_threshold
    }

    /// Leak the inner key and value
    #[inline]
    pub fn leak_rawkv(self) -> (Key, Value) {
        (self.key, self.val)
    }

    /// Get the encoded value.
    #[inline]
    pub fn encoded_value(&self) -> EncodedValue {
        self.val.to_encoded()
    }
}
