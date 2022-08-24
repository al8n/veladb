use crate::sync::Arc;
use vpb::{checksum::ChecksumVerificationMode, Compression, Encryption};
use zallocator::pool::AllocatorPool;

/// TableOptions contains configurable options for Table/Builder.
#[derive(Debug, Clone)]
pub struct TableOptions {
    // Options for Opening/Building Table.
    /// open table in read only mode
    ro: bool,
    metrics_enabled: bool,

    /// maximum size of the table
    table_size: u64,

    /// checksum_verification_mode is the checksum verification mode for Table.
    checksum_verification_mode: ChecksumVerificationMode,

    /// Indicates the compression algorithm used for block compression.
    compression: Compression,

    /// Indicates the encryption algorithm used for block encryption.
    encryption: Encryption,

    // Options for Table builder.
    /// The false positive probabiltiy of bloom filter.
    bloom_false_positive: f64,

    /// the size of each block inside SSTable in bytes.
    block_size: usize,

    /// Block cache is used to cache decompressed and decrypted blocks.
    // block_cache: Option<Arc<Cache>>,
    // index_cache: Option<Arc<Cache>>,
    alloc_pool: Arc<AllocatorPool>,
}

impl TableOptions {
    pub const fn default_with_pool(pool: Arc<AllocatorPool>) -> Self {
        Self {
            ro: false,
            metrics_enabled: true,
            table_size: 2 << 20,
            checksum_verification_mode: ChecksumVerificationMode::NoVerification,
            compression: Compression::new(),
            bloom_false_positive: 0.01,
            block_size: 4 * 1024,
            alloc_pool: pool,
            encryption: Encryption::new(),
        }
    }

    /// get whether read only or not
    #[inline]
    pub const fn read_only(&self) -> bool {
        self.ro
    }

    /// set whether read only or not
    #[inline]
    pub const fn set_read_only(mut self, value: bool) -> Self {
        self.ro = value;
        self
    }

    /// get if the metrics enabled
    #[inline]
    pub const fn metrics_enabled(&self) -> bool {
        self.metrics_enabled
    }

    /// set whether enable metrics or not
    #[inline]
    pub const fn set_metrics_enabled(mut self, value: bool) -> Self {
        self.metrics_enabled = value;
        self
    }

    /// get maximum size of the table
    #[inline]
    pub const fn table_size(&self) -> u64 {
        self.table_size
    }

    /// set maximum size of the table
    #[inline]
    pub const fn set_table_size(mut self, val: u64) -> Self {
        self.table_size = val;
        self
    }

    /// get maximum capacity of the table, 0.95x of the maximum size of the table
    #[cfg(feature = "nightly")]
    #[inline]
    pub const fn table_capacity(&self) -> u64 {
        (self.table_size as f64 * 0.95) as u64
    }

    /// get maximum capacity of the table, 0.9x of the maximum size of the table
    #[cfg(not(feature = "nightly"))]
    #[inline]
    pub fn table_capacity(&self) -> u64 {
        (self.table_size as f64 * 0.95) as u64
    }

    /// get the compression algorithm used for block compression.
    #[inline]
    pub const fn compression(&self) -> Compression {
        self.compression
    }

    /// set the compression algorithm used for block compression.
    #[inline]
    pub const fn set_compression(mut self, compression: Compression) -> Self {
        self.compression = compression;
        self
    }

    /// get the encryption algorithm used for block encryption.
    #[inline]
    pub const fn encryption(&self) -> &Encryption {
        &self.encryption
    }

    /// set the encryption algorithm used for block encryption.
    #[inline]
    pub fn set_encryption(mut self, encryption: Encryption) -> Self {
        self.encryption = encryption;
        self
    }

    /// get the checksum verification mode for `Table`.
    #[inline]
    pub const fn checksum_verification_mode(&self) -> ChecksumVerificationMode {
        self.checksum_verification_mode
    }

    /// set the checksum verification mode for `Table`.
    #[inline]
    pub const fn set_checksum_verification_mode(mut self, val: ChecksumVerificationMode) -> Self {
        self.checksum_verification_mode = val;
        self
    }

    /// get the false positive probabiltiy of bloom filter.
    #[inline]
    pub const fn bloom_ratio(&self) -> f64 {
        self.bloom_false_positive
    }

    /// set the false positive probabiltiy of bloom filter.
    #[inline]
    pub const fn set_bloom_ratio(mut self, val: f64) -> Self {
        self.bloom_false_positive = val;
        self
    }

    /// get the block size
    #[inline]
    pub const fn block_size(&self) -> usize {
        self.block_size
    }

    /// set the block size
    #[inline]
    pub const fn set_block_size(mut self, val: usize) -> Self {
        self.block_size = val;
        self
    }

    /// get the allocator pool
    #[inline]
    pub fn allocator_pool(&self) -> &AllocatorPool {
        &self.alloc_pool
    }

    /// set the allocator pool
    #[inline]
    pub fn set_allocator_pool(mut self, val: Arc<AllocatorPool>) -> Self {
        self.alloc_pool = val;
        self
    }
}
