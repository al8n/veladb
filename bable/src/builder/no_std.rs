use super::*;
use vpb::kvstructs::bytes::BytesMut;

impl Builder {
    pub fn new(opts: RefCounter<TableOptions>) -> Result<Self> {
        let sz = (2 * opts.table_size() as usize).min(MAX_ALLOCATOR_INITIAL_SIZE);

        let alloc = opts.allocator_pool().fetch(sz, "TableBuilder")?;
        alloc.set_tag("Builder");
        let cur_block = RefCounter::new(BBlock::new(
            alloc.allocate((opts.block_size() + PADDING) as u64)?,
        ));

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

    fn compress_data(alloc: &Allocator, data: Buffer, compression_algo: Compression) -> Buffer {
        if compression_algo.is_none() {
            return data;
        }

        let buffer = alloc.allocate_unchecked(data.max_encoded_len(compression_algo) as u64);
        let end = data
            .compress_to(buffer.as_mut_slice(), compression_algo)
            .map_err(|e| {
                #[cfg(feature = "tracing")]
                {
                    tracing::error!(target: "table_builder", err=%e, info = "error while compressing block in table builder.");
                }
                e
            })
            .unwrap();
        buffer.slice(..end)
    }

    fn encrypt_data(alloc: &Allocator, data: Buffer, encryption: &Encryption) -> Buffer {
        let algo = encryption.algorithm();
        match algo {
            #[cfg(any(feature = "aes", feature = "aes-std"))]
            EncryptionAlgorithm::Aes => {
                let iv = vpb::encrypt::random_iv();
                let key = encryption.secret();

                let buffer = alloc.allocate_unchecked((data.capacity() + iv.len()) as u64);
                let slice = buffer.as_mut_slice();
                data.encrypt_to(&mut slice[..data.capacity()], key, &iv, algo)
                    .map_err(|e| {
                        #[cfg(feature = "tracing")]
                        {
                            tracing::error!(target: "table_builder", err=%e, info = "error while encrypting block in table builder.");
                        }
                        e
                    })
                    .unwrap();
                slice[data.capacity()..].copy_from_slice(&iv);
                buffer
            }
            _ => data,
        }
    }
}

impl super::BuildData {
    #[inline]
    pub fn write(self, dst: &mut BytesMut) -> usize {
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
        written
    }
}
