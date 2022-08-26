use super::error::*;
use crate::{
    bloom::hash,
    sync::{Arc, AtomicU32, AtomicU8, AtomicUsize, Ordering},
};
use alloc::vec::Vec;
use core::{cell::UnsafeCell, ops::Deref, ptr::NonNull};
use vpb::kvstructs::{bytes::Bytes, Key, KeyExt, Value, ValueExt};
use zallocator::{pool::Allocator, Buffer};

#[cfg(feature = "std")]
mod standard;

#[cfg(not(feature = "std"))]
mod no_std;

const KB: usize = 1024;
const MB: usize = KB * 1024;
const MAX_ALLOCATOR_INITIAL_SIZE: usize = 256 << 20;

/// When a block is encrypted, it's length increases. We add 256 bytes of padding to
/// handle cases when block size increases. This is an approximate number.
const PADDING: usize = 256;

const HEADER_SIZE: usize = core::mem::size_of::<Header>();

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(C)]
pub(crate) struct Header {
    /// Overlap with base key.
    overlap: u16,
    /// Length of the diff.
    diff: u16,
}

impl Header {
    #[inline]
    pub fn encode(&self) -> [u8; HEADER_SIZE] {
        let mut buf = [0; HEADER_SIZE];
        buf[..2].copy_from_slice(&self.overlap.to_le_bytes());
        buf[2..].copy_from_slice(&self.diff.to_le_bytes());
        buf
    }

    #[inline]
    pub fn decode(buf: &[u8]) -> Self {
        Self {
            overlap: u16::from_le_bytes(buf[..2].try_into().unwrap()),
            diff: u16::from_le_bytes(buf[2..].try_into().unwrap()),
        }
    }
}

/// BBlock represents a block that is being compressed/encrypted in the background.
#[derive(Debug, Clone)]
#[repr(transparent)]
pub(crate) struct BBlock {
    inner: Arc<NonNull<BBlockInner>>,
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
        if Arc::count(&self.inner) == 1 {
            // Safety: the ptr will not be freed at this time.
            unsafe {
                core::ptr::drop_in_place(self.inner.as_ref().as_ptr());
            }
        }
    }
}

impl BBlock {
    #[inline]
    pub(crate) fn new(buf: Buffer) -> Self {
        Self {
            inner: Arc::new(unsafe {
                NonNull::new_unchecked(Box::into_raw(Box::new(BBlockInner::new(buf))))
            }),
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
pub struct Builder {
    /// Typically tens or hundreds of meg. This is for one single file.
    alloc: Allocator,
    cur_block: Arc<BBlock>,
    compressed_size: Arc<AtomicU32>,
    uncompressed_size: AtomicU32,

    len_offsets: u32,
    estimated_size: u32,
    key_hashes: Vec<u32>,
    opts: Arc<super::options::TableOptions>,
    max_version: u64,
    on_disk_size: u32,
    stale_data_size: usize,

    #[cfg(feature = "std")]
    wg: Option<crossbeam_utils::sync::WaitGroup>,
    #[cfg(feature = "std")]
    block_tx: Option<crossbeam_channel::Sender<Arc<BBlock>>>,
    block_list: Vec<Arc<BBlock>>,
}

impl Builder {
    /// Returns whether builder is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.key_hashes.len() == 0
    }

    /// Returns the number of blocks in the builder.
    #[inline]
    pub fn len(&self) -> usize {
        self.key_hashes.len()
    }

    /// Returns a suffix of newKey that is different from `self.base_key`.
    #[inline]
    fn key_diff<'a>(&'a self, new_key: &'a [u8]) -> &'a [u8] {
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

    fn add_helper(&mut self, key: Key, val: Value, vplen: u32) {
        self.key_hashes.push(hash(key.parse_key()));

        let version = key.parse_timestamp();
        if version > self.max_version {
            self.max_version = version;
        }

        let base_key = self.cur_block.base_key();
        // diffKey stores the difference of key with base key.
        let diff_key = if base_key.is_empty() {
            self.cur_block.set_base_key(key.clone());
            &key
        } else {
            self.key_diff(&key)
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
            tmp.copy_from_slice(data);
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

    fn close(self) {
        self.opts.allocator_pool().put(self.alloc);
    }
}
