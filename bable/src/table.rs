use crate::{
    builder::Builder,
    sync::{AtomicU64, Ordering},
    RefCounter,
};

use self::iterator::UniTableIterator;

use super::error::*;
use vpb::{
    checksum::calculate_checksum,
    kvstructs::{bytes::Bytes, Key},
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

#[cfg(feature = "std")]
mod standard;
#[cfg(feature = "std")]
use standard::*;

const FILE_SUFFIX: &str = "sst";
const INT_SIZE: usize = core::mem::size_of::<usize>();
static NUM_BLOCKS: AtomicU64 = AtomicU64::new(0);

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

    #[inline]
    pub fn path(&self) -> &std::path::Path {
        self.inner.path()
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
    pub fn iter(&self, opt: usize) -> UniTableIterator<RefCounter<RawTable>> {
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
