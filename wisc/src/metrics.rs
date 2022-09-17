use core::sync::atomic::AtomicU64;

/// Cumulative number of reads
pub static NUM_READS: AtomicU64 = AtomicU64::new(0);

/// Cumulative number of bytes read
pub static NUM_BYTES_READ: AtomicU64 = AtomicU64::new(0);
