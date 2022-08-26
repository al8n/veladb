impl Builder {
    pub fn new(opts: Arc<TableOptions>) -> Result<Self> {
        let sz = (2 * opts.table_size() as usize).min(MAX_ALLOCATOR_INITIAL_SIZE);

        let alloc = opts.allocator_pool().fetch(sz, "TableBuilder").unwrap();
        alloc.set_tag("Builder");
        let cur_block = Arc::new(BBlock::new(
            alloc.allocate((opts.block_size() + PADDING) as u64)?,
        ));

        Ok(Self {
            opts,
            alloc,
            cur_block,
            compressed_size,
            uncompressed_size: AtomicU32::new(0),
            len_offsets: 0,
            estimated_size: 0,
            key_hashes: Vec::new(),
            max_version: 0,
            on_disk_size: 0,
            stale_data_size: 0,
            block_list: Vec::new(),
        })
    }
}
