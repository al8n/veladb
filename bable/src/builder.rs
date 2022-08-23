use alloc::{sync::Arc, vec::Vec};
use core::cell::{Cell, UnsafeCell};
use core::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use kvstructs::{bytes::BytesMut, Key};
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
#[derive(Debug)]
pub(crate) struct BBlock {
    data: UnsafeCell<Buffer>,
    // Base key for the current block.
    base_key: Key,
    // Offsets of entries present in current block.
    entry_offsets: Vec<u32>,
    // Points to the end offset of the block.
    end: Cell<usize>,

    encrypt_data_ptr: *const u8,
    encrypt_data_len: usize,
}

impl BBlock {
    #[inline(always)]
    pub(crate) fn data(&self) -> &Buffer {
        let data = self.data.get();
        unsafe { &*data }
    }

    #[inline(always)]
    pub(crate) fn set_data(&self, data: Buffer) {
        let d = self.data.get();
        unsafe {
            *d = data;
        }
    }

    #[inline(always)]
    pub(crate) fn increase_end(&self, add: usize) {
        let x = self.end.get();
        self.end.set(x + add);
    }

    #[inline(always)]
    pub(crate) fn end(&self) -> usize {
        self.end.get()
    }

    #[inline(always)]
    pub(crate) fn set_end(&self, end: usize) {
        self.end.set(end);
    }
}

pub(crate) struct BBlockRef {
    ptr: core::ptr::NonNull<BBlock>,
    refs: Arc<AtomicUsize>,
}

impl BBlockRef {
    #[inline(always)]
    fn new(bblk: BBlock) -> Self {
        let bblk_ptr = Box::into_raw(Box::new(bblk));
        Self {
            ptr: unsafe { core::ptr::NonNull::new_unchecked(bblk_ptr) },
            // count self
            refs: Arc::new(AtomicUsize::new(1)),
        }
    }

    #[inline(always)]
    fn base_key(&self) -> &Key {
        &unsafe { self.ptr.as_ref() }.base_key
    }

    #[inline(always)]
    fn set_base_key(&self, key: Key) {
        let mut blk = unsafe { &mut *self.ptr.as_ptr() };
        blk.base_key = key;
    }

    #[inline(always)]
    fn encrypt_key(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self.encrypt_data_ptr, self.encrypt_data_len) }
    }

    #[inline(always)]
    const fn as_ptr(&self) -> *mut BBlock {
        self.ptr.as_ptr()
    }
}

impl core::ops::Deref for BBlockRef {
    type Target = BBlock;

    fn deref(&self) -> &Self::Target {
        unsafe { self.ptr.as_ref() }
    }
}

impl core::ops::DerefMut for BBlockRef {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.ptr.as_mut() }
    }
}

impl Clone for BBlockRef {
    fn clone(&self) -> Self {
        self.refs.fetch_add(1, Ordering::SeqCst);
        Self {
            ptr: self.ptr,
            refs: self.refs.clone(),
        }
    }
}

impl Drop for BBlockRef {
    fn drop(&mut self) {
        if self.refs.fetch_sub(1, Ordering::SeqCst) == 1 {
            unsafe {
                core::ptr::drop_in_place(self.ptr.as_ptr());
            }
        }
    }
}

unsafe impl Send for BBlockRef {}
unsafe impl Sync for BBlockRef {}

/// Builder is used in building a table.
pub struct Builder {
    /// Typically tens or hundreds of meg. This is for one single file.
    alloc: Arc<Allocator>,
    cur_block: BBlockRef,
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
    block_tx: Option<crossbeam_channel::Sender<BBlockRef>>,
    block_list: Vec<BBlockRef>,
}
