use super::*;
use crossbeam_channel::Receiver;
use crossbeam_utils::sync::WaitGroup;
use fmmap::{MmapFileMut, MmapFileMutExt};

impl Builder {
    #[inline]
    pub(super) fn new_in(opts: RefCounter<Options>) -> Result<Self> {
        let sz = (2 * opts.table_size() as usize).min(MAX_ALLOCATOR_INITIAL_SIZE);

        let alloc = opts.allocator_pool().fetch(sz, "TableBuilder")?;
        alloc.set_tag("Builder");
        let cur_block = BBlock::new(alloc.allocate((opts.block_size() + PADDING) as u64)?);

        // If encryption or compression is not enabled, do not start compression/encryption goroutines
        // and write directly to the buffer.
        if opts.compression().is_none() && opts.encryption().is_none() {
            return Ok(Builder {
                alloc,
                opts,
                cur_block,
                compressed_size: RefCounter::new(AtomicU32::new(0)),
                uncompressed_size: AtomicU32::new(0),
                len_offsets: 0,
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
        let compressed_size = RefCounter::new(AtomicU32::new(0));

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
            key_hashes: Vec::new(),
            max_version: 0,
            on_disk_size: 0,
            stale_data_size: 0,
            block_tx: Some(tx),
            block_list: Vec::new(),
            wg: Some(wg),
        })
    }
}

struct BlockProcessor {
    alloc: Allocator,
    compressed_size: RefCounter<AtomicU32>,
    wg: WaitGroup,
    rx: crossbeam_channel::Receiver<BBlock>,
    encryption: Encryption,
    compression: Compression,
}

impl BlockProcessor {
    fn new(
        alloc: Allocator,
        compressed_size: RefCounter<AtomicU32>,
        wg: WaitGroup,
        rx: Receiver<BBlock>,
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
                Builder::process_block(&alloc, item, &compressed_size, compression, &encryption)
            }
        });
    }
}

impl super::TableData for super::BuildData {
    #[inline]
    fn size(&self) -> usize {
        self.size as usize
    }

    fn write(self, dst: &mut MmapFileMut) -> Result<usize> {
        let mut written = 0;
        for blk in &self.block_list {
            written += MmapFileMutExt::write(dst, &blk.data().as_slice()[..blk.end()], written);
        }

        written += MmapFileMutExt::write(dst, self.index.as_slice(), written);

        written += MmapFileMutExt::write(
            dst,
            (self.index.len() as u32).to_be_bytes().as_ref(),
            written,
        );

        written += MmapFileMutExt::write(dst, self.checksum.as_slice(), written);

        written += MmapFileMutExt::write(dst, self.checksum_size.to_be_bytes().as_ref(), written);

        self.opts.allocator_pool().put(self.alloc);
        Ok(written)
    }
}
