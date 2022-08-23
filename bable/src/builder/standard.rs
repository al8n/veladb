use crossbeam_utils::sync::WaitGroup;
use std::thread::available_parallelism;
use vpb::{compression::Compressor, encrypt::Encryptor, Compression, DataKey, EncryptionAlgorithm};
use zallocator::Buffer;

use super::*;

impl Builder {
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
        self.opts
            .allocator_pool()
            .put(unsafe { core::ptr::read_volatile(Arc::into_raw(self.alloc)) });
    }
}

struct BlockProcessor {
    alloc: Arc<Allocator>,
    compressed_size: Arc<AtomicU32>,
    wg: WaitGroup,
    secret: Option<Arc<DataKey>>,
    rx: crossbeam_channel::Receiver<BBlockRef>,
    encrypt_buf: Buffer,
    encrypt_algo: EncryptionAlgorithm,
    compression_algo: Compression,
}

impl BlockProcessor {
    fn spawn(self) {
        std::thread::spawn(move || {
            for item in self.rx.iter() {
                let end = item.end();
                // Extract the block.
                let mut block_buf = item.data().slice(..end);
                // Compress the block.
                block_buf = self.compress_data(block_buf);
                // Encrypt the block.
                block_buf = self.encrypt_data(block_buf);

                // BlockBuf should always less than or equal to allocated space. If the blockBuf is greater
                // than allocated space that means the data from this block cannot be stored in its
                // existing location.
                let allocated_space =
                    vpb::compression::max_encoded_len(self.compression_algo, end) + PADDING + 1;
                let cap = block_buf.capacity();
                assert!(cap <= allocated_space);
                // blockBuf was allocated on allocator. So, we don't need to copy it over.
                item.set_data(block_buf);
                item.set_end(cap);
                self.compressed_size.fetch_add(cap as u32, Ordering::SeqCst);
            }
        });
    }

    fn compress_data(&self, data: Buffer) -> Buffer {
        if self.compression_algo.is_none() {
            return data;
        }

        let buffer = self
            .alloc
            .allocate_unchecked(data.max_encoded_len(self.compression_algo) as u64);
        let end = data
            .compress_to(buffer.as_mut_slice(), self.compression_algo)
            .expect("Error while compressing block in table builder.");
        buffer.slice(..end)
    }

    fn encrypt_data(&self, data: Buffer) -> Buffer {
        const DUMMY_SLICE: &[u8] = &[];

        match self.encrypt_algo {
            #[cfg(any(feature = "aes", feature = "aes-std"))]
            EncryptionAlgorithm::Aes => {
                let iv = vpb::encrypt::random_iv();
                let key = match self.secret {
                    Some(ref secret) => &secret.data,
                    None => DUMMY_SLICE,
                };

                let buffer = self
                    .alloc
                    .allocate_unchecked((data.capacity() + iv.len()) as u64);
                let slice = buffer.as_mut_slice();
                data.encrypt_to(&mut slice[..data.capacity()], key, &iv, self.encrypt_algo)
                    .expect("Error while encrypting block in table builder.");
                slice[data.capacity()..].copy_from_slice(&iv);
                buffer
            }
            _ => data,
        }
    }
}
