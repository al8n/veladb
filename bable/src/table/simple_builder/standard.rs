use fmmap::{MmapFileMut, MmapFileMutExt};

use super::Result;

impl super::TableData for super::SimpleBuildData {
    #[inline]
    fn size(&self) -> usize {
        self.buffer.capacity()
    }

    #[inline]
    fn write(self, dst: &mut MmapFileMut) -> Result<usize> {
        let written = MmapFileMutExt::write(dst, self.buffer.as_ref(), 0);
        self.opts.allocator_pool().put(self.alloc);
        Ok(written)
    }
}
