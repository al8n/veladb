use lazy_static::__Deref;
use vpb::{
    checksum::Checksumer,
    kvstructs::{KeyExt, ValueExt},
    BlockOffset, Checksum, TableIndex,
};
use zallocator::Buffer;

use crate::bloom::hash;

use super::*;

#[cfg(feature = "std")]
mod standard;
#[cfg(feature = "std")]
pub use standard::*;

#[cfg(not(feature = "std"))]
mod no_std;
#[cfg(not(feature = "std"))]
pub use no_std::*;

pub struct SimpleBuildData {
    alloc: Allocator,
    opts: RefCounter<Options>,
    buffer: Buffer,
}

/// `SimpleBuilder` is a faster builder for building table.
/// However, it can be only used when you do not want to encrypt or compress the data in the table.
/// If you want to encrypt or compress the data in the table, plese use [`Builder`] instead.
///
/// [`Builder`]: struct.Builder.html
pub struct SimpleBuilder {
    allocator: Allocator,
    buf: Buffer,
    base_key: Key,
    last_block_offset: u32,
    current_block_offset: u32,
    key_hashes: Vec<u32>,
    entry_offsets: Vec<u32>,
    table_index: TableIndex,
    stale_data_size: usize,
    max_version: u64,
    len_offsets: u32,
    opts: RefCounter<Options>,
}

impl super::TableBuilder for SimpleBuilder {
    type TableData = SimpleBuildData;

    fn options(&self) -> RefCounter<Options> {
        self.opts.clone()
    }

    #[inline]
    fn insert_stale(&mut self, key: &Key, val: &Value, value_len: u32) {
        // Rough estimate based on how much space it will occupy in the SST.
        self.stale_data_size += key.len()
            + val.len()
            + 4 // entry offset
            + 4; // header size

        self.insert_in(key, val, value_len, true)
    }

    #[inline]
    fn insert(&mut self, key: &Key, val: &Value, value_len: u32) {
        self.insert_in(key, val, value_len, false)
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.key_hashes.is_empty()
    }

    #[inline]
    fn len(&self) -> usize {
        self.key_hashes.len()
    }

    #[inline]
    fn reached_capacity(&self) -> bool {
        let block_size = self.current_block_offset
            + (self.entry_offsets.len() * 4) as u32
            + 4 // count of all entry offsets
            + Checksum::ENCODED_SIZE as u32; // checksum;

        let estimated_size = block_size
            + 4 // index length
            + self.len_offsets;

        (estimated_size as u64) > self.opts.table_capacity()
    }

    #[inline]
    fn build(self) -> Result<Option<Self::TableData>> {
        todo!()
    }
}

impl SimpleBuilder {
    #[inline]
    fn insert_in(&mut self, key: &Key, val: &Value, value_len: u32, is_stale: bool) {
        if self.should_finish_block(key.len(), val.encoded_size()) {
            if is_stale {
                // This key will be added to tableIndex and it is stale.
                self.stale_data_size += key.len()
                    + 4 // len
                    + 4; // offset
            }

            self.finish_block();
            self.base_key.clear();
            assert!(self.current_block_offset < u32::MAX);
            self.last_block_offset = self.current_block_offset;
            self.entry_offsets.clear();
        }

        self.insert_helper(key, val, value_len)
    }

    /// Structure of Block.
    ///  +-------------------+---------------------+--------------------+--------------+------------------+
    /// | Entry1            | Entry2              | Entry3             | Entry4       | Entry5           |
    ///  +-------------------+---------------------+--------------------+--------------+------------------+
    /// | Entry6            | ...                 | ...                | ...          | EntryN           |
    /// +-------------------+---------------------+--------------------+--------------+------------------+
    /// | Block Meta(contains list of offsets used| Block Meta Size    | Block        | Checksum Size    |
    /// | to perform binary search in the block)  | (4 Bytes)          | Checksum     | (4 Bytes)        |
    ///  +-----------------------------------------+--------------------+--------------+------------------+
    /// In case the data is encrypted, the "IV" is added to the end of the block.
    fn finish_block(&mut self) {
        let entries_len = self.entry_offsets.len();
        if entries_len == 0 {
            return;
        }

        let cur_block_end = self.current_block_offset as usize;

        self.append(u32_slice_to_bytes(&self.entry_offsets));
        self.append(&(entries_len as u32).to_be_bytes());

        // Append the block checksum and its length.
        let checksum = (&self.buf.as_slice()[self.last_block_offset as usize..cur_block_end])
            .checksum(Options::checksum(&self.opts))
            .marshal();
        self.append(&checksum);
        self.append(&(checksum.len() as u32).to_be_bytes());

        // Add length of baseKey (rounded to next multiple of 4 because of alignment).
        // Add another 40 Bytes, these additional 40 bytes consists of
        // 12 bytes of metadata of flatbuffer
        // 8 bytes for Key in flat buffer
        // 8 bytes for offset
        // 8 bytes for the len
        // 4 bytes for the size of slice while SliceAllocate
        self.len_offsets += (((self.base_key.len() as f64) / 4f64) * 4f64).ceil() as u32 + 40;

        self.table_index.offsets.push(BlockOffset {
            key: self.base_key.deref().clone(),
            offset: self.current_block_offset,
            len: self.current_block_offset - self.last_block_offset,
        });
    }

    fn should_finish_block(&self, key_size: usize, val_encoded_size: u32) -> bool {
        let len = self.entry_offsets.len();
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

        let estimated_size = (self.current_block_offset - self.last_block_offset)
            + 6 // header size for entry
            + key_size as u32
            + val_encoded_size
            + entires_offset_size;

        // Integer overflow check for table size.
        assert!(
            ((self.current_block_offset - self.last_block_offset) as u64) + (estimated_size as u64)
                < u32::MAX as u64
        );

        estimated_size > self.opts.block_size() as u32
    }

    /// Returns a suffix of newKey that is different from `self.base_key`.
    #[inline]
    fn key_diff(&self, new_key: &Key) -> Key {
        let (new_key_len, base_key_len) = (new_key.len(), self.base_key.len());
        let mut idx = 0;
        while idx < new_key_len && idx < base_key_len {
            if new_key[idx] != self.base_key[idx] {
                break;
            }
            idx += 1;
        }
        new_key.slice(idx..).into()
    }

    fn insert_helper(&mut self, key: &Key, val: &Value, vplen: u32) {
        self.key_hashes.push(hash(key.parse_key()));

        let version = key.parse_timestamp();
        if version > self.max_version {
            self.max_version = version;
        }

        // diffKey stores the difference of key with base key.
        let diff_key = if self.base_key.is_empty() {
            self.base_key = key.clone();
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
        // self.e(self.cur_block.end() as u32);
        let entry_offset = self.current_block_offset - self.last_block_offset;
        self.entry_offsets.push(entry_offset);

        // Layout: header, diff_key, value.
        self.append(&header.encode());
        self.append(diff_key);

        let val_encoded_size = val.encoded_size();
        let dst = self.allocate(val_encoded_size as usize);
        val.encode(dst.as_mut_slice());

        let sst_size = val_encoded_size + diff_key.len() as u32 + 4;
        self.table_index.estimated_size += sst_size as u32 + vplen;
    }

    fn allocate(&mut self, need: usize) -> Buffer {
        let prev_end = self.current_block_offset as usize;
        if self.buf.as_ref()[prev_end..].len() < need {
            // We need to reallocate. 1GB is the max size that the allocator can allocate.
            // While reallocating, if doubling exceeds that limit, then put the upper bound on it.
            let sz = (2 * (self.current_block_offset - self.last_block_offset) as u64)
                .min(zallocator::Zallocator::MAX_ALLOC)
                .max((prev_end + need) as u64);
            let tmp = self.allocator.allocate_unchecked(sz);
            unsafe {
                core::ptr::copy_nonoverlapping(
                    self.buf.as_ref()[..prev_end].as_ptr(),
                    tmp.as_mut_ptr(),
                    prev_end,
                );
            }
            self.buf = tmp;
        }

        self.current_block_offset += need as u32;
        self.buf.slice(prev_end..prev_end + need)
    }

    /// Appends to `cur_block.data`
    fn append(&mut self, data: &[u8]) {
        let dst = self.allocate(data.len());
        Buffer::copy_from_slice(&dst, data)
    }
}
