use fmmap::{MmapFileExt, MmapFileMut};
use vpb::{
    checksum::Checksumer,
    encrypt::Encryptor,
    kvstructs::{bytes::Bytes, Key, KeyExt},
    BlockOffset, Checksum, Compression, Marshaller, TableIndex,
};

use super::CheapIndex;
use crate::{bloom::MayContain, error::*, options::TableOptions, sync::AtomicU32, RefCounter};

pub(super) struct Inner {
    mmap: MmapFileMut,
    /// Initialized in OpenTable, using fd.Stat().
    table_size: usize,

    // Use fetch_index to access.
    index: RefCounter<TableIndex>,
    cheap: CheapIndex,

    // For file garbage collection.
    refs: AtomicU32,

    // The following are initialized once and const.
    /// Smallest keys (with timestamps).
    smallest: Key,
    /// Largest keys (with timestamps).
    biggest: Key,

    // file id, part of filename
    id: u64,

    checksum: Bytes,

    created_at: std::time::Instant,
    index_start: usize,
    index_len: usize,
    has_bloom_filter: bool,

    in_memory: bool,
    opts: RefCounter<TableOptions>,
}

impl Inner {
    #[inline]
    pub(super) fn max_version(&self) -> u64 {
        self.cheap.max_version
    }

    #[inline]
    pub(super) fn bloom_filter_size(&self) -> usize {
        self.cheap.bloom_filter_length
    }

    #[inline]
    pub(super) fn uncompressed_size(&self) -> u32 {
        self.cheap.uncompressed_size
    }

    #[inline]
    pub(super) fn key_count(&self) -> u32 {
        self.cheap.key_count
    }

    #[inline]
    pub(super) fn on_disk_size(&self) -> u32 {
        self.cheap.on_disk_size
    }

    #[inline]
    pub(super) fn secret(&self) -> &[u8] {
        self.opts.encryption().secret()
    }

    #[inline]
    pub(super) fn compression(&self) -> Compression {
        self.opts.compression()
    }

    /// Splits the table into at least n ranges based on the block offsets.
    pub(super) fn key_splits(&self, idx: usize, prefix: &[u8]) -> Vec<Key> {
        let mut res = Vec::new();
        if idx == 0 {
            return res;
        }

        let offsets_len = self.offsets_length();
        let jump = (offsets_len / idx).max(1);
        let mut idx = 0;

        while idx < offsets_len {
            if idx >= offsets_len {
                idx = offsets_len - 1;
            }
            let bo = self.offsets(idx).unwrap();

            if bo.key.has_prefix(prefix) {
                res.push(bo.key.clone().into());
            }
            idx += offsets_len;
        }
        res
    }

    #[inline]
    fn cheap_index(&self) -> &CheapIndex {
        &self.cheap
    }

    #[inline]
    fn offsets_length(&self) -> usize {
        self.cheap.offsets_length
    }

    #[inline(always)]
    fn read(&self, offset: usize, sz: usize) -> Result<&[u8]> {
        self.mmap.bytes(offset, sz).map_err(|e| e.into())
    }

    #[inline(always)]
    fn read_no_fail(&self, offset: usize, sz: usize) -> &[u8] {
        self.mmap.bytes(offset, sz).unwrap()
    }

    /// Returns true if and only if the table does not have the key hash.
    /// It does a bloom filter lookup.
    #[inline]
    pub(super) fn contains_hash(&self, hash: u32) -> bool {
        if !self.has_bloom_filter {
            return false;
        }

        #[cfg(feature = "metrics")]
        {
            crate::metrics::add_bloom_hits(crate::metrics::DOES_NOT_HAVE_ALL, 1)
        }
        let index = self.fetch_index();
        let may_contain = index.bloom_filter.may_contain(hash);
        #[cfg(feature = "metrics")]
        {
            if !may_contain {
                crate::metrics::add_bloom_hits(crate::metrics::DOES_NOT_HAVE_HIT, 1)
            }
        }

        return !may_contain;
    }

    /// Returns true if all the keys in the table are prefixed by the given prefix.
    #[inline]
    pub(super) fn covered_by_prefix(&self, prefix: &[u8]) -> bool {
        self.biggest.parse_key().has_prefix(prefix) && self.smallest.parse_key().has_prefix(prefix)
    }

    fn init_biggest_and_smallest(&mut self) {
        match self.init_index() {
            Ok(ko) => {
                self.smallest = Key::from(ko.key);
                // TODO: iter to find the biggest
            }
            Err(e) => {
                // This defer will help gathering debugging info incase initIndex crashes.
                #[cfg(feature = "tracing")]
                scopeguard::defer_on_unwind! {
                    // Get the count of null bytes at the end of file. This is to make sure if there was an
                    // issue with mmap sync or file copy.
                    let mut count = 0;
                    for i in self.mmap.len() - 1 ..=0 {
                        if self.mmap.as_slice()[i] != 0 {
                            break;
                        }
                        count += 1;
                    }

                    {
                        tracing::info!("== Recovering from initIndex crash ==");
                        tracing::info!("File info: [id: {}, size: {}, zeros: {}]", self.id, self.table_size, count);
                        tracing::info!("is_encrypt: {}", self.should_decrypt());

                        let mut read_pos = self.table_size;

                        // Read checksum size.
                        read_pos -= 4;
                        let buf = self.read_no_fail(read_pos, 4);
                        let checksum_len = u32::from_be_bytes(buf.try_into().unwrap());
                        tracing::info!("checksum length: {}", checksum_len);

                        // Read checksum
                        read_pos -= checksum_len as usize;
                        let buf = self.read_no_fail(read_pos, checksum_len as usize);
                        let checksum: Checksum = Marshaller::unmarshal(buf).unwrap();
                        tracing::info!("checksum: {:?}", checksum);

                        // Read index size from the footer.
                        read_pos -= 4;
                        let buf = self.read_no_fail(read_pos, 4);
                        let index_len = u32::from_be_bytes(buf.try_into().unwrap());
                        tracing::info!("index len: {}", index_len);

                        // Read index.
                        read_pos -= 4;
                        self.index_start = read_pos;
                        let index_data = self.read_no_fail(read_pos, self.index_len);
                        tracing::info!("index: {:?}", index_data);
                    }
                };

                #[cfg(feature = "tracing")]
                {
                    tracing::error!(target: "table", info = "fail to init biggest and smallest index for table", err = %e);
                }
                panic!("{}", e)
            }
        }
    }

    /// init_index reads the index and populate the necessary table fields
    fn init_index(&mut self) -> Result<BlockOffset> {
        let mut read_pos = self.table_size;
        // read checksum len from the last 4 bytes.
        read_pos -= 4;
        let buf = self.read_no_fail(read_pos, 4);
        let cks_len = u32::from_be_bytes(buf.try_into().unwrap()) as usize;

        // read checksum
        read_pos -= cks_len;
        let buf = self.read_no_fail(read_pos, cks_len);
        let cks = Checksum::unmarshal(buf)?;

        // read index size from the footer
        read_pos -= 4;
        let buf = self.read_no_fail(read_pos, 4);
        self.index_len = u32::from_be_bytes(buf.try_into().unwrap()) as usize;

        // read index
        read_pos -= self.index_len;
        self.index_start = read_pos;
        let data = self.read_no_fail(read_pos, self.index_len);
        if !data.verify_checksum(cks.sum, TableOptions::checksum(&self.opts)) {
            return Err(Error::ChecksumMismatch);
        }
        let index = self.read_table_index()?;
        let bo = index.offsets[0].clone();
        let has_bloom_filter = !index.bloom_filter.is_empty();
        self.cheap = CheapIndex {
            max_version: index.max_version,
            key_count: index.key_count,
            uncompressed_size: index.uncompressed_size,
            on_disk_size: self.table_size as u32,
            bloom_filter_length: index.bloom_filter.len(),
            offsets_length: index.offsets.len(),
            num_entries: index.key_count as usize,
        };

        if !self.should_decrypt() {
            // If there's no encryption, this points to the mmap'ed buffer.
            self.index = RefCounter::new(index);
        }
        self.has_bloom_filter = has_bloom_filter;
        Ok(bo)
    }

    fn fetch_index(&self) -> RefCounter<TableIndex> {
        if !self.should_decrypt() {
            return self.index.clone();
        }

        match self.opts.index_cache() {
            Some(cache) => match cache.get(&self.id) {
                Some(index) => index.as_ref().clone(),
                None => {
                    let index = self.read_table_index().map(RefCounter::new).map_err(|e| {
                            #[cfg(feature = "tracing")]
                            {
                                tracing::error!(target: "table", info = "fail to read table idex", err = %e);
                            }
                            e
                        }).unwrap();
                    cache.insert(self.id, index.clone(), self.index_len as i64);
                    index
                }
            },
            None => {
                panic!("Index Cache must be set for encrypted workloads");
            }
        }
    }

    fn offsets(&self, idx: usize) -> Option<&BlockOffset> {
        self.fetch_index().offsets.get(idx)
    }

    /// read_table_index reads table index from the sst and returns its pb format.
    pub(super) fn read_table_index(&self) -> Result<TableIndex> {
        let buf = self.read_no_fail(self.index_start, self.index_len);

        // Decrypt the table index if it is encrypted.
        if self.should_decrypt() {
            self.decrypt(buf)
                .and_then(|data| TableIndex::unmarshal(data.as_slice()).map_err(From::from))
        } else {
            TableIndex::unmarshal(buf).map_err(From::from)
        }
    }

    #[inline]
    fn should_decrypt(&self) -> bool {
        self.opts.encryption().is_some()
    }

    #[inline]
    fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
        let encryption = self.opts.encryption();
        let block_size = encryption.block_size();
        // Last BlockSize bytes of the data is the IV.
        let iv = &data[data.len() - block_size..];
        // reset all bytes are data.
        let data = &data[..data.len() - block_size];

        data.encrypt_to_vec(encryption.secret(), iv, encryption.algorithm())
            .map_err(From::from)
    }
}
