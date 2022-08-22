use core::ops::{Deref, DerefMut};
use zallocator::Buffer;

use super::*;

impl Builder {
    pub(crate) fn allocate(&self, need: usize) -> Buffer {
        let data = self.cur_block.data();
        let prev_end = self.cur_block.end();
        if data.deref()[prev_end..].len() < need {
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
    pub(crate) fn append(&self, data: &[u8]) {
        let dst = self.allocate(data.len());
        Buffer::copy_from_slice(&dst, data)
    }
}
