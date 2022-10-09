use bable::{bytes::Bytes, kvstructs::KeyExt};

pub struct IteratorOptions {
    /// The number of KV pairs to prefetch while iterating.
    /// Valid only if `prefetch_values` is true.
    pub prefetch_size: usize,
    /// Indicates whether we should prefetch values during
    /// iteration and store them.
    pub prefetch_values: bool,
    /// Direction of iteration. False is forward, true is backward.
    pub reverse: bool,
    /// Fetch all valid versions of the same key.
    pub all_versions: bool,
    /// Used to allow internal access to wisc keys.
    pub internal_access: bool,

    // The following option is used to narrow down the SSTables that iterator
    // picks up. If Prefix is specified, only tables which could have this
    // prefix are picked based on their range of keys.
    /// If set, use the prefix for bloom filter lookup.
    prefix_is_key: bool,
    /// Only iterate over this given prefix.
    pub prefix: Bytes,
    /// Only read data that has version > `since_timpestamp`.
    pub since_timpestamp: u64,
}

impl Default for IteratorOptions {
    fn default() -> Self {
        Self {
            prefetch_size: 100,
            prefetch_values: true,
            reverse: false,
            all_versions: false,
            internal_access: false,
            prefix_is_key: false,
            prefix: Bytes::new(),
            since_timpestamp: 0,
        }
    }
}

impl IteratorOptions {
    #[inline]
    fn compare_to_prefix(&self, key: impl KeyExt) -> core::cmp::Ordering {
        // We should compare key without timestamp. For example key - a[TS] might be > "aa" prefix.
        let mut key = key.parse_key();
        if key.len() > self.prefix.len() {
            key = &key[..self.prefix.len()];
        }

        key.cmp(&self.prefix)
    }
}
