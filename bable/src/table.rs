use crate::RefCounter;

use self::iterator::UniTableIterator;

use super::error::*;
use alloc::vec::Vec;
use vpb::{
    checksum::calculate_checksum,
    kvstructs::{
        bytes::{BufMut, Bytes},
        Key, Value,
    },
    ChecksumAlgorithm, Compression, Marshaller,
};

mod iterator;
pub use iterator::*;

#[cfg(test)]
mod test;

#[cfg(not(feature = "std"))]
mod no_std;
#[cfg(not(feature = "std"))]
use no_std::*;

// #[cfg(feature = "std")]
// mod no_std;
// #[cfg(feature = "std")]
// use no_std::*;

#[cfg(feature = "std")]
mod standard;
#[cfg(feature = "std")]
use standard::*;

mod builder;
pub use builder::*;

mod simple_builder;
pub use simple_builder::*;

mod options;
pub use options::*;
use zallocator::pool::Allocator;

const FILE_SUFFIX: &str = "sst";
const INT_SIZE: usize = core::mem::size_of::<usize>();

///
///
/// You can also implement your own table builder, but you must make sure:
///
/// 1. The structure of block is the same as:
///
///     ```text
///     +-------------------+---------------------+--------------------+--------------+------------------+
///     | Entry1            | Entry2              | Entry3             | Entry4       | Entry5           |
///     +-------------------+---------------------+--------------------+--------------+------------------+
///     | Entry6            | ...                 | ...                | ...          | EntryN           |
///     +-------------------+---------------------+--------------------+--------------+------------------+
///     | Block Meta(contains list of offsets used| Block Meta Size    | Block        | Checksum Size    |
///     | to perform binary search in the block)  | (4 Bytes)          | Checksum     | (4 Bytes)        |
///     +-----------------------------------------+--------------------+--------------+------------------+
///     ```
///     In case the data is encrypted, the "IV" is added to the end of the block.
/// 2. a
pub trait TableBuilder {
    type TableData: TableData;

    fn new(opts: RefCounter<Options>) -> Result<Self>
    where
        Self: Sized;

    fn options(&self) -> RefCounter<Options>;

    /// The same as `insert` function but it also increments the internal
    /// `stale_data_size` counter. This value will be used to prioritize this table for
    /// compaction.
    fn insert_stale(&mut self, key: &Key, val: &Value, value_len: u32);

    /// Inserts a key-value pair to the block.
    fn insert(&mut self, key: &Key, val: &Value, value_len: u32);

    /// Returns whether builder is empty.
    fn is_empty(&self) -> bool;

    /// Returns the number of blocks in the builder.
    fn len(&self) -> usize;

    /// Returns true if we... roughly (?) reached capacity?
    fn reached_capacity(&self) -> bool;

    /// Build the data to be written to the table file.
    fn build(self) -> Result<Option<Self::TableData>>;
}

pub const HEADER_SIZE: usize = core::mem::size_of::<Header>();

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
    pub fn encode_to_buf(&self, mut buf: impl BufMut) {
        buf.put_u16_le(self.overlap);
        buf.put_u16_le(self.diff);
    }

    #[inline]
    pub fn decode(buf: &[u8]) -> Self {
        Self {
            overlap: u16::from_le_bytes(buf[..2].try_into().unwrap()),
            diff: u16::from_le_bytes(buf[2..4].try_into().unwrap()),
        }
    }

    #[inline]
    pub fn overlap(&self) -> u16 {
        self.overlap
    }

    #[inline]
    pub fn diff(&self) -> u16 {
        self.diff
    }
}

pub struct Block {
    offset: usize,
    data: Bytes,
    checksum: Bytes,
    /// start index of entry_offsets list
    entries_index_start: usize,
    /// used to binary search an entry in the block.
    pub(crate) entry_offsets: Vec<u32>,
}

impl Block {
    #[inline(always)]
    pub fn data(&self) -> Bytes {
        self.data.slice(..self.entries_index_start)
    }

    #[inline(always)]
    pub fn entry_offsets(&self) -> &[u32] {
        self.entry_offsets.as_slice()
    }

    #[inline(always)]
    pub fn size(&self) -> u64 {
        (3 * INT_SIZE // Size of the offset, entriesIndexStart and chkLen
            + self.data.len() + self.checksum.len() + self.entry_offsets.capacity() * 4)
            as u64
    }

    #[inline]
    pub fn verify_checksum(&self, algo: ChecksumAlgorithm) -> Result<()> {
        if calculate_checksum(&self.data, algo)
            .marshal()
            .ne(self.checksum.as_ref())
        {
            return Err(Error::ChecksumMismatch);
        }
        Ok(())
    }
}

#[derive(Copy, Clone)]
struct CheapIndex {
    max_version: u64,
    key_count: u32,
    uncompressed_size: u32,
    on_disk_size: u32,
    bloom_filter_length: usize,
    offsets_length: usize,
    num_entries: usize,
}

#[repr(transparent)]
pub struct Table {
    inner: RefCounter<RawTable>,
}

impl Table {
    #[inline]
    pub fn biggest(&self) -> &Key {
        self.inner.biggest()
    }

    #[inline]
    pub fn smallest(&self) -> &Key {
        self.inner.smallest()
    }

    #[inline]
    pub fn id(&self) -> u64 {
        self.inner.id()
    }

    #[cfg(feature = "std")]
    #[inline]
    pub fn path(&self) -> &std::path::Path {
        self.inner.path()
    }

    #[cfg(not(feature = "std"))]
    #[inline]
    pub fn path(&self) -> &str {
        // self.inner.path()
        todo!()
    }

    #[inline]
    pub fn max_version(&self) -> u64 {
        self.inner.max_version()
    }

    #[inline]
    pub fn bloom_filter_size(&self) -> usize {
        self.inner.bloom_filter_size()
    }

    #[inline]
    pub fn uncompressed_size(&self) -> u32 {
        self.inner.uncompressed_size()
    }

    #[inline]
    pub fn key_count(&self) -> u32 {
        self.inner.key_count()
    }

    #[inline]
    pub fn on_disk_size(&self) -> u32 {
        self.inner.on_disk_size()
    }

    #[inline]
    pub fn secret(&self) -> &[u8] {
        self.inner.secret()
    }

    #[inline]
    pub fn compression(&self) -> Compression {
        self.inner.compression()
    }

    #[inline]
    pub fn checksum(&self) -> &[u8] {
        self.inner.checksum()
    }

    #[inline]
    pub fn checksum_bytes(&self) -> &Bytes {
        self.inner.checksum_bytes()
    }

    #[cfg(feature = "std")]
    #[inline]
    pub fn in_memory(&self) -> bool {
        self.inner.in_memory()
    }

    #[inline]
    pub fn key_splits(&self, id: usize, prefix: &[u8]) -> Vec<Key> {
        self.inner.key_splits(id, prefix)
    }

    #[inline]
    pub fn contains_hash(&self, hash: u32) -> bool {
        self.inner.contains_hash(hash)
    }

    #[inline]
    pub fn covered_by_prefix(&self, prefix: &[u8]) -> bool {
        self.inner.covered_by_prefix(prefix)
    }

    #[inline]
    pub fn iter(&self, opt: Flag) -> UniTableIterator<RefCounter<RawTable>> {
        UniTableIterator::new(self.inner.clone(), opt)
    }
}

impl From<RawTable> for Table {
    fn from(inner: RawTable) -> Self {
        Self {
            inner: RefCounter::new(inner),
        }
    }
}

impl core::ops::Deref for Table {
    type Target = RefCounter<RawTable>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl AsRef<RawTable> for Table {
    fn as_ref(&self) -> &RawTable {
        &self.inner
    }
}

#[inline(always)]
fn u32_slice_to_bytes(bytes: &[u32]) -> &[u8] {
    const DUMMY: &[u8] = &[];
    if bytes.is_empty() {
        return DUMMY;
    }

    let len = bytes.len();
    let ptr = bytes.as_ptr();
    // Safety:
    // - This function is not exposed to the public.
    unsafe { core::slice::from_raw_parts(ptr.cast::<u8>(), len * 4) }
}
