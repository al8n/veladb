use crossbeam_channel::Receiver;
use crossbeam_utils::sync::WaitGroup;
use vpb::{
    compression::Compressor, encrypt::Encryptor, Compression, Encryption, EncryptionAlgorithm,
};
use zallocator::Buffer;

use crate::options::TableOptions;

use super::*;

impl Builder {
    pub fn new(opts: Arc<TableOptions>) -> Result<Self> {
        let sz = (2 * opts.table_size() as usize).min(MAX_ALLOCATOR_INITIAL_SIZE);

        let alloc = opts.allocator_pool().fetch(sz, "TableBuilder").unwrap();
        alloc.set_tag("Builder");
        let cur_block = Arc::new(BBlock::new(
            alloc.allocate((opts.block_size() + PADDING) as u64)?,
        ));

        // If encryption or compression is not enabled, do not start compression/encryption goroutines
        // and write directly to the buffer.
        if opts.compression().is_none() && opts.encryption().is_none() {
            return Ok(Builder {
                alloc,
                opts,
                cur_block,
                compressed_size: Arc::new(AtomicU32::new(0)),
                uncompressed_size: AtomicU32::new(0),
                len_offsets: 0,
                estimated_size: 0,
                key_hashes: Vec::new(),
                max_version: 0,
                on_disk_size: 0,
                stale_data_size: 0,
                block_tx: None,
                block_list: Vec::new(),
                wg: None,
            });
        }

        let count = num_cpus::get() * 2;
        let (tx, rx) = crossbeam_channel::bounded(count * 2);
        let compressed_size = Arc::new(AtomicU32::new(0));

        let wg = WaitGroup::new();
        for _ in 0..count {
            BlockProcessor::new(
                alloc.clone(),
                compressed_size.clone(),
                wg.clone(),
                rx.clone(),
                opts.encryption().clone(),
                opts.compression(),
            )
            .spawn();
        }

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
            block_tx: Some(tx),
            block_list: Vec::new(),
            wg: Some(wg),
        })
    }

    pub(crate) fn allocate(&self, need: usize) -> Buffer {
        let data = self.cur_block.data();
        let prev_end = self.cur_block.end();
        if data.as_ref()[prev_end..].len() < need {
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

    pub(crate) fn close(self) {
        self.opts.allocator_pool().put(self.alloc);
    }
}

struct BlockProcessor {
    alloc: Allocator,
    compressed_size: Arc<AtomicU32>,
    wg: WaitGroup,
    rx: crossbeam_channel::Receiver<Arc<BBlock>>,
    encryption: Encryption,
    compression: Compression,
}

impl BlockProcessor {
    fn new(
        alloc: Allocator,
        compressed_size: Arc<AtomicU32>,
        wg: WaitGroup,
        rx: Receiver<Arc<BBlock>>,
        encryption: Encryption,
        compression: Compression,
    ) -> Self {
        Self {
            alloc,
            compressed_size,
            wg,
            rx,
            encryption,
            compression,
        }
    }

    fn spawn(self) {
        std::thread::spawn(move || {
            #[cfg(feature = "tracing")]
            {
                tracing::debug!(target: "table_builder", info = "starting block processor...");
            }

            let Self {
                alloc,
                compressed_size,
                wg,
                rx,
                encryption,
                compression,
            } = self;
            scopeguard::defer!(wg.wait());
            for item in rx.iter() {
                let end = item.end();
                // Extract the block.
                let mut block_buf = item.data().slice(..end);
                // Compress the block.
                block_buf = Self::compress_data(&alloc, block_buf, compression);
                // Encrypt the block.
                block_buf = Self::encrypt_data(&alloc, block_buf, &encryption);

                // BlockBuf should always less than or equal to allocated space. If the blockBuf is greater
                // than allocated space that means the data from this block cannot be stored in its
                // existing location.
                let allocated_space =
                    vpb::compression::max_encoded_len(self.compression, end) + PADDING + 1;
                let cap = block_buf.capacity();
                assert!(cap <= allocated_space);
                // blockBuf was allocated on allocator. So, we don't need to copy it over.
                item.set_data(block_buf);
                item.set_end(cap);
                compressed_size.fetch_add(cap as u32, Ordering::SeqCst);
            }
        });
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
