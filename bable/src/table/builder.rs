use super::*;
use crate::{
    bloom::{bloom_bits_per_key, hash, Filter},
    sync::{AtomicU32, Ordering},
};
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::{ops::Deref, ptr::NonNull};
use vpb::{
    checksum::Checksumer,
    compression::Compressor,
    encrypt::{random_iv, Encryptor},
    kvstructs::{Key, KeyExt, Value, ValueExt},
    BlockOffset, Checksum, Compression, Encryption, Marshaller, TableIndex,
};
use zallocator::Buffer;

#[cfg(feature = "std")]
mod standard;

#[cfg(not(feature = "std"))]
mod no_std;

pub const MAX_ALLOCATOR_INITIAL_SIZE: usize = 256 << 20;

/// When a block is encrypted, it's length increases. We add 256 bytes of padding to
/// handle cases when block size increases. This is an approximate number.
const PADDING: usize = 256;

pub struct BuildData {
    alloc: Allocator,
    opts: RefCounter<Options>,
    block_list: Vec<BBlock>,
    index: Vec<u8>,
    checksum: Vec<u8>,
    checksum_size: u32,
    size: u32,
}

/// BBlock represents a block that is being compressed/encrypted in the background.
#[derive(Debug, Clone)]
#[repr(transparent)]
pub(crate) struct BBlock {
    inner: RefCounter<NonNull<BBlockInner>>,
}

/// Safety: we never concurrently read/write data in `BBlock`
unsafe impl Send for BBlock {}
unsafe impl Sync for BBlock {}

struct BBlockInner {
    data: Buffer,
    /// Base key for the current block.
    base_key: Key,
    /// Offsets of entries present in current block.
    entry_offsets: Vec<u32>,
    /// Points to the end offset of the block.
    end: usize,
}

impl BBlockInner {
    #[inline]
    pub(crate) const fn new(buf: Buffer) -> Self {
        Self {
            data: buf,
            base_key: Key::new(),
            entry_offsets: Vec::new(),
            end: 0,
        }
    }
}

impl Drop for BBlock {
    fn drop(&mut self) {
        if RefCounter::count(&self.inner) == 1 && !self.inner.as_ptr().is_null() {
            // Safety: the ptr will not be freed at this time.
            unsafe {
                core::ptr::drop_in_place(self.inner.as_ptr());
            }
        }
    }
}

impl Default for BBlock {
    fn default() -> Self {
        Self::dangling()
    }
}

impl BBlock {
    #[inline(always)]
    pub(crate) fn new(buf: Buffer) -> Self {
        Self {
            inner: RefCounter::new(unsafe {
                NonNull::new_unchecked(Box::into_raw(Box::new(BBlockInner::new(buf))))
            }),
        }
    }

    #[inline(always)]
    pub(crate) fn dangling() -> Self {
        Self {
            inner: RefCounter::new(unsafe { NonNull::new_unchecked(core::ptr::null_mut()) }),
        }
    }

    #[inline(always)]
    pub(crate) fn data(&self) -> &Buffer {
        &self.inner().data
    }

    #[inline(always)]
    pub(crate) fn set_data(&self, data: Buffer) {
        self.inner_mut().data = data;
    }

    #[inline(always)]
    pub(crate) fn increase_end(&self, add: usize) -> usize {
        let end = self.inner_mut().end;
        self.inner_mut().end += add;
        end
    }

    #[inline(always)]
    pub(crate) fn end(&self) -> usize {
        self.inner().end
    }

    #[inline(always)]
    pub(crate) fn set_end(&self, end: usize) {
        self.inner_mut().end = end;
    }

    #[inline(always)]
    pub(crate) fn base_key(&self) -> &Key {
        &self.inner().base_key
    }

    #[inline(always)]
    pub(crate) fn set_base_key(&self, key: Key) {
        self.inner_mut().base_key = key;
    }

    #[inline(always)]
    pub(crate) fn entry_offsets(&self) -> &[u32] {
        &self.inner().entry_offsets
    }

    #[inline(always)]
    pub(crate) fn len(&self) -> usize {
        self.inner().entry_offsets.len()
    }

    #[inline(always)]
    pub(crate) fn push_entry_offset(&self, offset: u32) {
        self.inner_mut().entry_offsets.push(offset);
    }

    #[inline(always)]
    fn inner(&self) -> &BBlockInner {
        // Safety: the ptr will not be freed at this time.
        unsafe { &*(self.inner.deref().as_ptr()) }
    }

    #[inline(always)]
    #[allow(clippy::mut_from_ref)]
    fn inner_mut(&self) -> &mut BBlockInner {
        // Safety: the ptr will not be freed at this time.
        unsafe { &mut *(self.inner.deref().as_ptr()) }
    }
}

/// Builder is used in building a table.
///
/// If you do not want to compress or encrypt data,
/// Please use [`SimpleBuilder`], it is faster than `Builder`
///
/// [`SimpleBuilder`]: struct.SimpleBuilder.html
pub struct Builder {
    /// Typically tens or hundreds of meg. This is for one single file.
    alloc: Allocator,
    cur_block: BBlock,
    #[cfg(feature = "std")]
    compressed_size: RefCounter<AtomicU32>,
    #[cfg(not(feature = "std"))]
    compressed_size: AtomicU32,
    uncompressed_size: AtomicU32,

    len_offsets: u32,
    key_hashes: Vec<u32>,
    opts: RefCounter<super::options::Options>,
    max_version: u64,
    on_disk_size: u32,
    stale_data_size: usize,

    #[cfg(feature = "std")]
    wg: Option<crossbeam_utils::sync::WaitGroup>,
    #[cfg(feature = "std")]
    block_tx: Option<crossbeam_channel::Sender<BBlock>>,
    block_list: Vec<BBlock>,
}

impl TableBuilder for Builder {
    type TableData = BuildData;

    fn new(opts: RefCounter<Options>) -> Result<Self>
    where
        Self: Sized,
    {
        Builder::new_in(opts)
    }

    fn options(&self) -> RefCounter<Options> {
        self.opts.clone()
    }

    /// The same as `insert` function but it also increments the internal
    /// `stale_data_size` counter. This value will be used to prioritize this table for
    /// compaction.
    fn insert_stale(&mut self, key: &Key, val: &Value, value_len: u32) {
        // Rough estimate based on how much space it will occupy in the SST.
        self.stale_data_size += key.len()
            + val.len()
            + 4 // entry offset
            + 4; // header size

        self.insert_in(key, val, value_len, true)
    }

    /// Inserts a key-value pair to the block.
    fn insert(&mut self, key: &Key, val: &Value, value_len: u32) {
        self.insert_in(key, val, value_len, false)
    }

    /// Returns whether builder is empty.
    #[inline]
    fn is_empty(&self) -> bool {
        self.key_hashes.len() == 0
    }

    /// Returns the number of blocks in the builder.
    #[inline]
    fn len(&self) -> usize {
        self.key_hashes.len()
    }

    // TODO: vvv this was the comment on ReachedCapacity.
    // FinalSize returns the *rough* final size of the array, counting the header which is
    // not yet written.
    // TODO: Look into why there is a discrepancy. I suspect it is because of Write(empty, empty)
    // at the end. The diff can vary.
    fn reached_capacity(&self) -> bool {
        // If encryption/compression is enabled then use the compresssed size.
        let mut sum_block_size = self.compressed_size.load(Ordering::SeqCst);
        if self.opts.compression().is_none() && self.opts.encryption().is_none() {
            sum_block_size = self.uncompressed_size.load(Ordering::SeqCst);
        }

        let block_size = sum_block_size
            + (self.cur_block.len() * core::mem::size_of::<u32>()) as u32
            + 4 // count of all entry offsets
            + Checksum::ENCODED_SIZE as u32; // checksum;

        let estimated_size = block_size
            + 4 // index length
            + self.len_offsets;

        (estimated_size as u64) > self.opts.table_capacity()
    }

    fn build(mut self) -> Result<Option<BuildData>> {
        self.finish_block(false);
        #[cfg(feature = "std")]
        {
            if self.block_tx.is_some() {
                self.block_tx.take();
                // Wait for block handler to finish.
                self.wg.unwrap().wait();
            }
        }

        if self.block_list.is_empty() {
            return Ok(None);
        }

        let key_hashes_len = self.key_hashes.len();
        let mut tbi = TableIndex::new();
        if self.opts.bloom_ratio() > 0.0 {
            let bits = bloom_bits_per_key(key_hashes_len, self.opts.bloom_ratio());
            tbi.bloom_filter = Filter::new(self.key_hashes.as_slice(), bits).into_bytes();
        }

        let mut num_entries = 0;
        let mut data_sz = 0;
        tbi.offsets = self
            .block_list
            .iter()
            .map(|bblk| {
                num_entries += bblk.len();
                let end = bblk.end() as u32;
                let bo = BlockOffset {
                    key: bblk.base_key().deref().clone(),
                    offset: data_sz,
                    len: end,
                };
                data_sz += end;
                bo
            })
            .collect();

        self.on_disk_size += data_sz;
        tbi.uncompressed_size = self.uncompressed_size.load(Ordering::SeqCst);
        tbi.key_count = key_hashes_len as u32;
        tbi.max_version = self.max_version;
        tbi.stale_data_size = self.stale_data_size as u32;

        let data = tbi.marshal();
        let encryption = self.opts.encryption();

        if encryption.is_some() {
            let index = data.as_slice().encrypt_to_vec(
                encryption.secret(),
                &random_iv(),
                encryption.algorithm(),
            )?;
            // Build checksum for the index.
            let cks = index
                .as_slice()
                .checksum(Options::checksum(&self.opts))
                .marshal();
            let cks_size = cks.len();
            let size = data_sz + (index.len() + cks_size + 4 + 4) as u32;

            Ok(Some(BuildData {
                alloc: self.alloc,
                opts: self.opts,
                block_list: self.block_list,
                index,
                checksum: cks,
                checksum_size: cks_size as u32,
                size,
            }))
        } else {
            let cks = data
                .as_slice()
                .checksum(Options::checksum(&self.opts))
                .marshal();

            let cks_size = cks.len();
            let size = data_sz + (data.len() + cks_size + 4 + 4) as u32;
            Ok(Some(BuildData {
                alloc: self.alloc,
                opts: self.opts,
                block_list: self.block_list,
                index: data,
                checksum: cks,
                checksum_size: cks_size as u32,
                size,
            }))
        }
    }
}

impl Builder {
    #[inline]
    fn insert_in(&mut self, key: &Key, val: &Value, value_len: u32, is_stale: bool) {
        if self.should_finish_block(key.len(), val.encoded_size()) {
            if is_stale {
                // This key will be added to tableIndex and it is stale.
                self.stale_data_size += key.len()
                    + 4 // len
                    + 4; // offset
            }

            self.finish_block(true)
        }

        self.insert_helper(key, val, value_len)
    }

    /// # Structure of Block.
    ///
    /// ```text
    /// +-------------------+---------------------+--------------------+--------------+------------------+
    /// | Entry1            | Entry2              | Entry3             | Entry4       | Entry5           |
    /// +-------------------+---------------------+--------------------+--------------+------------------+
    /// | Entry6            | ...                 | ...                | ...          | EntryN           |
    /// +-------------------+---------------------+--------------------+--------------+------------------+
    /// | Block Meta(contains list of offsets used| Block Meta Size    | Block        | Checksum Size    |
    /// | to perform binary search in the block)  | (4 Bytes)          | Checksum     | (4 Bytes)        |
    /// +-----------------------------------------+--------------------+--------------+------------------+
    /// ```
    /// In case the data is encrypted, the "IV" is added to the end of the block.
    fn finish_block(&mut self, start_new: bool) {
        let entries = self.cur_block.entry_offsets();
        let entries_len = entries.len();
        if entries_len == 0 {
            return;
        }

        let cur_block_end = self.cur_block.end();

        self.append(u32_slice_to_bytes(entries));
        self.append(&(entries_len as u32).to_be_bytes());

        // Append the block checksum and its length.
        let checksum = (&self.cur_block.data().as_slice()[..cur_block_end])
            .checksum(Options::checksum(&self.opts))
            .marshal();
        self.append(&checksum);
        self.append(&(checksum.len() as u32).to_be_bytes());

        self.uncompressed_size
            .fetch_add(cur_block_end as u32, Ordering::SeqCst);

        // Add length of baseKey (rounded to next multiple of 4 because of alignment).
        // Add another 40 Bytes, these additional 40 bytes consists of
        // 12 bytes of metadata of flatbuffer
        // 8 bytes for Key in flat buffer
        // 8 bytes for offset
        // 8 bytes for the len
        // 4 bytes for the size of slice while SliceAllocate
        self.len_offsets +=
            (((self.cur_block.base_key().len() as f64) / 4f64) * 4f64).ceil() as u32 + 40;

        self.block_list.push(self.cur_block.clone());

        let old_block = if start_new {
            core::mem::replace(
                &mut self.cur_block,
                BBlock::new(
                    self.alloc
                        .allocate_unchecked((self.opts.block_size() + PADDING) as u64),
                ),
            )
        } else {
            core::mem::take(&mut self.cur_block)
        };
        #[cfg(feature = "std")]
        {
            // If compression/encryption is enabled, we need to send the block to the blockChan.
            if let Some(ref tx) = self.block_tx {
                if let Err(e) = tx.send(old_block) {
                    #[cfg(feature = "tracing")]
                    tracing::error!(target: "table_builder", info = "fail send bblock to the processor", err = ?e);
                    panic!("fail send bblock to the processor: {e}");
                }
            }
        }

        // if no_std, encrypt and compress data in current thread
        #[cfg(not(feature = "std"))]
        {
            if self.opts.compression().is_some() || self.opts.encryption().is_some() {
                Self::process_block(
                    &self.alloc,
                    old_block,
                    &self.compressed_size,
                    self.opts.compression(),
                    self.opts.encryption(),
                )
            }
        }
    }

    fn should_finish_block(&self, key_size: usize, val_encoded_size: u32) -> bool {
        let current_block_end = self.cur_block.end() as u32;
        let len = self.cur_block.len();
        // If there is no entry till now, we will return false.
        if len == 0 {
            return false;
        }

        // Integer overflow check for statements below.
        assert!(((len as u32) + 1) * 4 + 4 + (Checksum::ENCODED_SIZE as u32) < u32::MAX);
        // We should include current entry also in size, that's why +1 to len(b.entryOffsets).
        let entires_offset_size = ((len as u32) + 1)
            * 4
            + 4 // size of entires list
            + (Checksum::ENCODED_SIZE as u32);

        let estimated_size = current_block_end
            + 6 // header size for entry
            + key_size as u32
            + val_encoded_size
            + entires_offset_size
            // IV is added at the end of the block, while encrypting.
		    // So, size of IV is added to estimatedSize.
            + self.opts.encryption().block_size() as u32;

        // Integer overflow check for table size.
        assert!((current_block_end as u64) + (estimated_size as u64) < u32::MAX as u64);

        estimated_size > self.opts.block_size() as u32
    }

    /// Returns a suffix of newKey that is different from `self.base_key`.
    #[inline]
    fn key_diff<'a>(&self, new_key: &'a [u8]) -> &'a [u8] {
        let base_key = self.cur_block.base_key();
        let (new_key_len, base_key_len) = (new_key.len(), base_key.len());
        let mut idx = 0;
        while idx < new_key_len && idx < base_key_len {
            if new_key[idx] != base_key[idx] {
                break;
            }
            idx += 1;
        }
        &new_key[idx..]
    }

    fn insert_helper(&mut self, key: &Key, val: &Value, vplen: u32) {
        self.key_hashes.push(hash(key.parse_key()));

        let version = key.parse_timestamp();
        if version > self.max_version {
            self.max_version = version;
        }

        let base_key = self.cur_block.base_key();
        // diffKey stores the difference of key with base key.
        let diff_key = if base_key.is_empty() {
            self.cur_block.set_base_key(key.clone());
            key
        } else {
            self.key_diff(key)
        };

        let key_len = key.len();
        let diff_key_len = diff_key.len();
        assert!(key_len - diff_key_len <= u16::MAX as usize);
        assert!(diff_key_len <= u16::MAX as usize);

        let header = Header {
            overlap: (key_len - diff_key_len) as u16,
            diff: diff_key_len as u16,
        };

        // store current entry's offset
        self.cur_block
            .push_entry_offset(self.cur_block.end() as u32);

        // Layout: header, diff_key, value.
        self.append(&header.encode());
        self.append(diff_key);

        let dst = self.allocate(val.encoded_size() as usize);
        val.encode(dst.as_mut_slice());

        // Add the vpLen to the onDisk size. We'll add the size of the block to
        // onDisk size in Finish() function.
        self.on_disk_size += vplen;
    }

    fn allocate(&self, need: usize) -> Buffer {
        let data = self.cur_block.data();
        let prev_end = self.cur_block.end();
        if data.as_ref()[prev_end..].len() < need {
            // We need to reallocate. 1GB is the max size that the allocator can allocate.
            // While reallocating, if doubling exceeds that limit, then put the upper bound on it.
            let sz = (2 * data.len() as u64)
                .min(zallocator::Zallocator::MAX_ALLOC)
                .max((prev_end + need) as u64);
            let tmp = self.alloc.allocate_unchecked(sz);
            unsafe {
                core::ptr::copy_nonoverlapping(
                    data.as_ref()[..prev_end].as_ptr(),
                    tmp.as_mut_ptr(),
                    prev_end,
                );
            }
            self.cur_block.set_data(tmp);
        }

        self.cur_block.increase_end(need);
        self.cur_block.data().slice(prev_end..prev_end + need)
    }

    /// Appends to `cur_block.data`
    fn append(&self, data: &[u8]) {
        let dst = self.allocate(data.len());
        Buffer::copy_from_slice(&dst, data)
    }

    #[inline(always)]
    fn process_block(
        alloc: &Allocator,
        item: BBlock,
        compressed_size: &AtomicU32,
        compression: Compression,
        encryption: &Encryption,
    ) {
        let end = item.end();
        // Extract the block.
        let mut block_buf = item.data().slice(..end);
        // Compress the block.
        block_buf = Self::compress_data(alloc, block_buf, compression);
        // Encrypt the block.
        block_buf = Self::encrypt_data(alloc, block_buf, encryption);

        // BlockBuf should always less than or equal to allocated space. If the blockBuf is greater
        // than allocated space that means the data from this block cannot be stored in its
        // existing location.
        let allocated_space = vpb::compression::max_encoded_len(compression, end) + PADDING + 1;
        let cap = block_buf.capacity();
        assert!(cap <= allocated_space);
        // blockBuf was allocated on allocator. So, we don't need to copy it over.
        item.set_data(block_buf);
        item.set_end(cap);
        compressed_size.fetch_add(cap as u32, Ordering::SeqCst);
    }

    #[inline(always)]
    fn compress_data(alloc: &Allocator, data: Buffer, compression_algo: Compression) -> Buffer {
        if compression_algo.is_none() {
            return data;
        }

        let buffer = alloc.allocate_unchecked(data.max_encoded_len(compression_algo) as u64);
        let end = data
            .compress_to(buffer.as_mut_slice(), compression_algo)
            .map_err(|e| {
                #[cfg(feature = "tracing")]
                {
                    tracing::error!(target: "table_builder", err=%e, info = "error while compressing block in table builder.");
                }
                e
            })
            .unwrap();
        buffer.slice(..end)
    }

    #[inline(always)]
    fn encrypt_data(_alloc: &Allocator, data: Buffer, encryption: &Encryption) -> Buffer {
        let algo = encryption.algorithm();
        match algo {
            #[cfg(any(feature = "aes", feature = "aes-std"))]
            vpb::EncryptionAlgorithm::Aes => {
                let iv = vpb::encrypt::random_iv();
                let key = encryption.secret();

                let buffer = _alloc.allocate_unchecked((data.capacity() + iv.len()) as u64);
                let slice = buffer.as_mut_slice();
                data.encrypt_to(&mut slice[..data.capacity()], key, &iv, algo)
                    .map_err(|e| {
                        #[cfg(feature = "tracing")]
                        {
                            tracing::error!(target: "table_builder", err=%e, info = "error while encrypting block in table builder.");
                        }
                        e
                    })
                    .unwrap();
                slice[data.capacity()..].copy_from_slice(&iv);
                buffer
            }
            _ => data,
        }
    }
}
