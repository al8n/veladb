#![cfg_attr(not(feature = "std"), no_std)]

use core::time::Duration;
use std::path::PathBuf;

use vpb::{kvstructs::bytes::Bytes, Encryption};

pub use vpb;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeyRegistryOptions {
    pub dir: PathBuf,
    pub in_memory: bool,
    pub read_only: bool,
    pub encryption_key_rotation_duration: Duration,
    pub encryption: Encryption,
}

impl Default for KeyRegistryOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyRegistryOptions {
    #[inline]
    pub fn new() -> Self {
        Self {
            dir: std::env::temp_dir().join("registry"),
            in_memory: false,
            read_only: false,
            encryption_key_rotation_duration: Duration::from_secs(0),
            encryption: Encryption::new(),
        }
    }

    #[inline]
    pub const fn dir(&self) -> &PathBuf {
        &self.dir
    }

    #[inline]
    pub fn set_dir(mut self, dir: PathBuf) -> Self {
        self.dir = dir;
        self
    }

    #[inline]
    pub const fn set_in_memory(mut self) -> Self {
        self.in_memory = true;
        self
    }

    #[inline]
    pub const fn in_memory(&self) -> bool {
        self.in_memory
    }

    #[inline]
    pub const fn set_read_only(mut self) -> Self {
        self.read_only = true;
        self
    }

    #[inline]
    pub const fn read_only(&self) -> bool {
        self.read_only
    }

    #[inline]
    pub const fn set_encryption_key_rotation_duration(mut self, duration: Duration) -> Self {
        self.encryption_key_rotation_duration = duration;
        self
    }

    #[inline]
    pub const fn encryption_key_rotation_duration(&self) -> Duration {
        self.encryption_key_rotation_duration
    }

    #[inline]
    pub fn set_encryption(mut self, encryption: Encryption) -> Self {
        self.encryption = encryption;
        self
    }

    #[inline]
    pub const fn encryption(&self) -> &Encryption {
        &self.encryption
    }

    #[inline]
    pub fn secret(&self) -> &[u8] {
        self.encryption.secret()
    }

    #[inline]
    pub fn secret_bytes(&self) -> Bytes {
        self.encryption.secret_bytes()
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct MemTableOptions {
    pub size: u64,
    pub in_memory: bool,
    pub read_only: bool,
}

impl MemTableOptions {
    #[inline]
    pub const fn set_size(mut self, size: u64) -> Self {
        self.size = size;
        self
    }

    #[inline]
    pub const fn size(&self) -> u64 {
        self.size
    }

    #[inline]
    pub const fn set_in_memory(mut self) -> Self {
        self.in_memory = true;
        self
    }

    #[inline]
    pub const fn in_memory(&self) -> bool {
        self.in_memory
    }

    #[inline]
    pub const fn set_read_only(mut self) -> Self {
        self.read_only = true;
        self
    }

    #[inline]
    pub const fn read_only(&self) -> bool {
        self.read_only
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LogFileOptions {
    pub sync_writes: bool,
    pub read_only: bool,
    pub enable_metrics: bool,
    pub mem_table_size: u64,
    pub save_on_drop: bool,
}

impl<'a> From<&'a ValueLogOptions> for LogFileOptions {
    fn from(opts: &'a ValueLogOptions) -> Self {
        Self {
            sync_writes: opts.sync_writes,
            read_only: opts.read_only,
            enable_metrics: opts.enable_metrics,
            mem_table_size: 2 * opts.file_size,
            save_on_drop: opts.save_on_drop,
        }
    }
}

impl From<ValueLogOptions> for LogFileOptions {
    fn from(opts: ValueLogOptions) -> Self {
        Self {
            sync_writes: opts.sync_writes,
            read_only: opts.read_only,
            enable_metrics: opts.enable_metrics,
            mem_table_size: 2 * opts.file_size,
            save_on_drop: opts.save_on_drop,
        }
    }
}

impl LogFileOptions {
    #[inline]
    pub const fn sync_writes(&self) -> bool {
        self.sync_writes
    }

    #[inline]
    pub const fn set_sync_writes(mut self, sync_writes: bool) -> Self {
        self.sync_writes = sync_writes;
        self
    }

    #[inline]
    pub const fn read_only(&self) -> bool {
        self.read_only
    }

    #[inline]
    pub const fn set_read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    #[inline]
    pub const fn enable_metrics(&self) -> bool {
        self.enable_metrics
    }

    #[inline]
    pub const fn set_enable_metrics(mut self, enable_metrics: bool) -> Self {
        self.enable_metrics = enable_metrics;
        self
    }

    #[inline]
    pub const fn mem_table_size(&self) -> u64 {
        self.mem_table_size
    }

    #[inline]
    pub const fn set_mem_table_size(mut self, mem_table_size: u64) -> Self {
        self.mem_table_size = mem_table_size;
        self
    }

    #[inline]
    pub const fn save_on_drop(&self) -> bool {
        self.save_on_drop
    }

    #[inline]
    pub const fn set_save_on_drop(mut self, save_on_drop: bool) -> Self {
        self.save_on_drop = save_on_drop;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ValueLogOptions {
    pub size: u64,
    pub in_memory: bool,
    pub read_only: bool,
    pub sync_writes: bool,
    pub enable_metrics: bool,
    pub dir_path: PathBuf,
    pub file_size: u64,
    pub save_on_drop: bool,
    pub verify_value_checksum: bool,
    /// Transaction start and commit timestamps are managed by end-user.
    /// This is only useful for databases built on top of VelaDB.
    /// Not recommended for most users.
    pub managed_txns: bool,
    pub max_entries: u32,
    pub max_batch_size: u64,
    pub max_batch_count: u64,
}

impl ValueLogOptions {
    #[inline]
    pub const fn size(&self) -> u64 {
        self.size
    }

    #[inline]
    pub const fn set_size(mut self, size: u64) -> Self {
        self.size = size;
        self
    }

    #[inline]
    pub const fn in_memory(&self) -> bool {
        self.in_memory
    }

    #[inline]
    pub const fn set_in_memory(mut self, in_memory: bool) -> Self {
        self.in_memory = in_memory;
        self
    }

    #[inline]
    pub const fn read_only(&self) -> bool {
        self.read_only
    }

    #[inline]
    pub const fn set_read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    #[inline]
    pub const fn sync_writes(&self) -> bool {
        self.sync_writes
    }

    #[inline]
    pub const fn set_sync_writes(mut self, sync_writes: bool) -> Self {
        self.sync_writes = sync_writes;
        self
    }

    #[inline]
    pub const fn enable_metrics(&self) -> bool {
        self.enable_metrics
    }

    #[inline]
    pub const fn set_enable_metrics(mut self, enable_metrics: bool) -> Self {
        self.enable_metrics = enable_metrics;
        self
    }

    #[inline]
    pub const fn dir_path(&self) -> &PathBuf {
        &self.dir_path
    }

    #[inline]
    pub fn set_dir_path(mut self, dir_path: PathBuf) -> Self {
        self.dir_path = dir_path;
        self
    }

    #[inline]
    pub const fn file_size(&self) -> u64 {
        self.file_size
    }

    #[inline]
    pub const fn set_file_size(mut self, file_size: u64) -> Self {
        self.file_size = file_size;
        self
    }

    #[inline]
    pub const fn save_on_drop(&self) -> bool {
        self.save_on_drop
    }

    #[inline]
    pub const fn set_save_on_drop(mut self, save_on_drop: bool) -> Self {
        self.save_on_drop = save_on_drop;
        self
    }

    #[inline]
    pub const fn verify_value_checksum(&self) -> bool {
        self.verify_value_checksum
    }

    #[inline]
    pub const fn set_verify_value_checksum(mut self, verify_value_checksum: bool) -> Self {
        self.verify_value_checksum = verify_value_checksum;
        self
    }

    #[inline]
    pub const fn managed_txns(&self) -> bool {
        self.managed_txns
    }

    #[inline]
    pub const fn set_managed_txns(mut self, managed_txns: bool) -> Self {
        self.managed_txns = managed_txns;
        self
    }

    #[inline]
    pub const fn max_entries(&self) -> u32 {
        self.max_entries
    }

    #[inline]
    pub const fn set_max_entries(mut self, max_entries: u32) -> Self {
        self.max_entries = max_entries;
        self
    }

    #[inline]
    pub const fn max_batch_size(&self) -> u64 {
        self.max_batch_size
    }

    #[inline]
    pub const fn set_max_batch_size(mut self, max_batch_size: u64) -> Self {
        self.max_batch_size = max_batch_size;
        self
    }

    #[inline]
    pub const fn max_batch_count(&self) -> u64 {
        self.max_batch_count
    }

    #[inline]
    pub const fn set_max_batch_count(mut self, max_batch_count: u64) -> Self {
        self.max_batch_count = max_batch_count;
        self
    }
}
