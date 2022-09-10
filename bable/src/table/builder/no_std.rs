use super::*;

impl Builder {
    pub(super) fn new_in(opts: RefCounter<Options>) -> Result<Self> {
        let sz = (2 * opts.table_size() as usize).min(MAX_ALLOCATOR_INITIAL_SIZE);

        let alloc = opts.allocator_pool().fetch(sz, "TableBuilder")?;
        alloc.set_tag("Builder");
        let cur_block = BBlock::new(alloc.allocate((opts.block_size() + PADDING) as u64)?);

        Ok(Self {
            opts,
            alloc,
            cur_block,
            compressed_size: AtomicU32::new(0),
            uncompressed_size: AtomicU32::new(0),
            len_offsets: 0,
            key_hashes: Vec::new(),
            max_version: 0,
            on_disk_size: 0,
            stale_data_size: 0,
            block_list: Vec::new(),
        })
    }
}

impl super::TableData for super::BuildData {
    #[inline]
    fn size(&self) -> usize {
        self.size as usize
    }

    #[inline]
    fn write(self, mut dst: impl BufMut) -> Result<usize> {
        let mut written = 0;
        for blk in &self.block_list {
            let end = blk.end();
            written += end;
            dst.put_slice(&blk.data().as_slice()[..end]);
        }

        let index_slice = self.index.as_slice();
        written += index_slice.len();
        dst.put_slice(index_slice);

        written += core::mem::size_of::<u32>();
        dst.put_u32(index_slice.len() as u32);

        let checksum_slice = self.checksum.as_slice();
        written += checksum_slice.len();

        dst.put_slice(checksum_slice);

        written += core::mem::size_of::<u32>();
        dst.put_u32(self.checksum_size);

        self.opts.allocator_pool().put(self.alloc);
        Ok(written)
    }
}
