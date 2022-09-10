use super::*;

impl SimpleBuilder {
    pub(super) fn new_in(opts: RefCounter<Options>) -> Result<Self> {
        if opts.compression().is_some() || opts.encryption().is_some() {
            panic!("Please use TableBuilder when enable encryption or compression.");
        }

        Ok(Self {
            buf: BytesMut::with_capacity((16 << 20) + opts.table_size() as usize),
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
    fn write(self, mut dst: impl BufMut) -> Result<usize> {
        dst.put_slice(self.data.as_ref());
        Ok(self.data.len())
    }
}
