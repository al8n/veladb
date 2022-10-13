use bable::{bytes::Bytes, kvstructs::KeyExt, Table};

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

    #[inline]
    pub(crate) fn pick_table(&self, t: &Table) -> bool {
        // Ignore this table if its max version is less than the sinceTs.
        if t.max_version() < self.since_timpestamp {
            return false;
        }

        if self.prefix.is_empty() {
            return true;
        }

        if matches!(self.compare_to_prefix(t.smallest()), core::cmp::Ordering::Greater) {
            return false;
        }

        if matches!(self.compare_to_prefix(t.biggest()), core::cmp::Ordering::Less) {
            return false;
        }

        // Bloom filter lookup would only work if opt.Prefix does NOT have the read
	    // timestamp as part of the key.
        if self.prefix_is_key && t.contains_hash(bable::bloom::hash(&self.prefix)) {
            return false;
        }

        true
    }

    pub(crate) fn pick_tables<'a>(&self, all_tables: &'a [Table]) -> Vec<Table> {
        if self.prefix.is_empty() {
           return self.filter_tables(all_tables);
        }

        let s_idx = indexsort::search(all_tables.len(), |i| {
            !matches!(self.compare_to_prefix(all_tables[i].biggest()), core::cmp::Ordering::Less)
        });

        if s_idx == all_tables.len() {
            return Vec::new();
        }

        let filtered = &all_tables[s_idx..];
        if !self.prefix_is_key {
            let e_idx = indexsort::search(filtered.len(), |i| {
                matches!(self.compare_to_prefix(filtered[i].smallest()), core::cmp::Ordering::Greater)
            });

            return self.filter_tables(&filtered[..e_idx]);
        }

        // self.prefix_is_key == true. This code is optimizing for opt.prefix_is_key part.
        let hash = bable::bloom::hash(&self.prefix);
        
        let return_all = self.since_timpestamp == 0;

        let mut vec = vec![];
        for t in filtered {
            // When we encounter the first table whose smallest key is higher than self.prefix, we can
		    // stop. This is an IMPORTANT optimization, just considering how often we call
		    // NewKeyIterator.
            if matches!(self.compare_to_prefix(t.smallest()), core::cmp::Ordering::Greater) {
                // if table.smallest() > self.prefix, then this and all tables after this can be ignored.
                break;
            }

            // self.prefix is actually the key. So, we can run bloom filter checks
		    // as well.
            if t.contains_hash(hash) {
                continue;
            }

            if return_all {
                vec.push(t.clone());
                continue;
            }

           
            if t.max_version() >= self.since_timpestamp {
                vec.push(t.clone());
            }
        }

        vec
    }

    fn filter_tables<'a>(&self, tables: impl IntoIterator<Item = &'a Table>) -> Vec<Table> {
        if self.since_timpestamp == 0 {
            return tables.into_iter().cloned().collect();
        }

        tables.into_iter().filter_map(|t| {
            if t.max_version() >= self.since_timpestamp {
                Some(t.clone())
            } else {
                None
            }
        }).collect()
    }
}
