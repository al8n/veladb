use super::error::*;
use kvstructs::bytes::Bytes;
use vpb::{checksum::calculate_checksum, ChecksumAlgorithm, Marshaller};

const INT_SIZE: usize = core::mem::size_of::<usize>();

pub struct Block {
    offset: usize,
    data: Bytes,
    checksum: Bytes,
    /// start index of entry_offsets list
    entries_index_start: usize,
    /// used to binary search an entry in the block.
    pub(crate) entry_offsets: Vec<u32>,

    /// checksum length.
    cks_len: usize,
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
