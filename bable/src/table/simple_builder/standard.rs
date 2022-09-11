use fmmap::{MmapFileMut, MmapFileMutExt};

use super::*;

impl SimpleBuilder {
    pub(super) fn new_in(opts: RefCounter<Options>) -> Result<Self> {
        if opts.compression().is_some() || opts.encryption().is_some() {
            panic!("Please use TableBuilder when enable encryption or compression.");
        }

        Ok(Self {
            buf: BytesMut::with_capacity(
                ((2 * opts.table_size()) as usize).min(MAX_ALLOCATOR_INITIAL_SIZE),
            ),
            base_key: Default::default(),
            last_block_offset: 0,
            key_hashes: Default::default(),
            entry_offsets: Default::default(),
            table_index: Default::default(),
            stale_data_size: 0,
            max_version: 0,
            len_offsets: 0,
            opts,
        })
    }
}

impl super::TableData for super::SimpleBuildData {
    #[inline]
    fn size(&self) -> usize {
        self.data.len()
    }

    #[inline]
    fn write(self, dst: &mut MmapFileMut) -> Result<usize> {
        let written = MmapFileMutExt::write(dst, self.data.as_ref(), 0);
        Ok(written)
    }
}
