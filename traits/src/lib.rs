#![cfg_attr(not(feature = "std"), no_std)]

use std::path::PathBuf;

use vela_options::{
    vpb::{
        kvstructs::{
            bytes::{Bytes, BytesMut},
            request::Request,
            Entry, EntryRef, Key, Value, ValuePointer,
        },
        DataKey,
    },
    KeyRegistryOptions, LogFileOptions, MemTableOptions, ValueLogOptions,
};
use vela_utils::ref_counter::RefCounter;

pub trait KeyRegistry: Clone + Sized + Send + Sync + 'static {
    type Error: std::error::Error;

    /// Opens key registry if it exists, otherwise it'll create key registry
    /// and returns key registry.
    fn open(opts: RefCounter<KeyRegistryOptions>) -> Result<Self, Self::Error>;

    /// Rewrite the existing key registry file with new one.
    /// It is okay to give closed key registry. Since, it's using only the datakey.
    fn rewrite(&self, opts: RefCounter<KeyRegistryOptions>) -> Result<(), Self::Error>;

    /// Returns `DataKey` of the given key id.
    fn data_key(&self, id: &u64) -> Result<Option<DataKey>, Self::Error>;

    /// Returns the number of data keys in the registry.
    fn num_data_keys(&self) -> usize;

    /// Give you the latest generated datakey based on the rotation
    /// period. If the last generated datakey lifetime exceeds the rotation period.
    /// It'll create new datakey.
    fn latest_data_key(&self) -> Result<DataKey, Self::Error>;

    /// Inserts a new datakey into the registry
    fn insert(&self, dk: DataKey) -> Result<u64, Self::Error>;
}

pub trait LogFile: Sized + Send + Sync + 'static {
    type Error: std::error::Error
        + From<<Self::KeyRegistry as KeyRegistry>::Error>
        + Into<<Self::KeyRegistry as KeyRegistry>::Error>;
    type KeyRegistry: KeyRegistry;

    fn open<P: AsRef<std::path::Path>>(
        path: P,
        #[cfg(unix)] flags: i32,
        #[cfg(windows)] flags: u32,
        fid: u32,
        registry: Self::KeyRegistry,
        opts: LogFileOptions,
    ) -> Result<Self, Self::Error>;

    /// Syncs the LogFile file.
    fn sync(&self) -> Result<(), Self::Error>;

    /// Returns the current wal file stat.
    fn stat(&self) -> Result<fmmap::MetaData, Self::Error>;

    /// Truncates the LogFile file.
    fn truncate(&self, end: u64) -> Result<(), Self::Error>;

    // /// Read from the log file
    // fn read(&self, p: ValuePointer) -> Result<Vec<u8>, Self::Error>;

    /// Writes the entry to the LogFile file.
    fn write_entry(&self, buf: &mut BytesMut, ent: &Entry) -> Result<(), Self::Error>;

    /// Encode entry to the buf layout of entry
    ///
    /// ```text
    /// +--------+-----+-------+-------+
    /// | header | key | value | crc32 |
    /// +--------+-----+-------+-------+
    /// ```
    fn encode_entry(
        &self,
        buf: &mut BytesMut,
        ent: &Entry,
        offset: u32,
    ) -> Result<usize, Self::Error>;

    /// Decode [`Entry`] from src
    fn decode_entry(&self, src: &[u8], offset: u32) -> Result<Entry, Self::Error>;

    /// Iterates over wal file. It doesn't not allocate new memory for every kv pair.
    /// Therefore, the kv pair is only valid for the duration of fn call.
    fn iter<F: FnMut(EntryRef, ValuePointer) -> Result<(), Self::Error>>(
        &self,
        offset: u32,
        log_entry: F,
    ) -> Result<u32, Self::Error>;
}

pub trait MemoryTable: Sized + Send + Sync + 'static {
    type Error: std::error::Error;

    fn sync_wal(&self) -> Result<(), Self::Error>;

    fn is_full(&self) -> bool;

    fn insert(&mut self, key: Key, val: Value) -> Result<(), Self::Error>;

    fn update_skiplist(&self) -> Result<(), Self::Error>;
}

pub trait Oracle: Sized + Send + Sync + 'static {
    fn read_timestamp(&self) -> u64;
}

pub trait ValueLog: Sized + Send + Sync + 'static {
    type Database: Database<KeyRegistry = <Self::LogFile as LogFile>::KeyRegistry>;
    type GC: ValueLogGC<Error = Self::Error>;
    type LogFile: LogFile;
    type Error: std::error::Error + From<<Self::Database as Database>::Error>;

    fn open(
        db: RefCounter<Self::Database>,
        opts: ValueLogOptions,
    ) -> Result<Option<Self>, Self::Error>;

    /// Create and append new log file
    fn append(&self) -> Result<RefCounter<Self::LogFile>, Self::Error>;

    fn create(
        dir_path: &PathBuf,
        registry: <Self::LogFile as LogFile>::KeyRegistry,
        max_fid: u32,
        opts: LogFileOptions,
    ) -> core::result::Result<Self::LogFile, Self::Error>;

    /// Reads the value log at a given location.
    fn read(&self, vp: ValuePointer) -> Result<Bytes, Self::Error>;

    fn write(&self, reqs: &mut [Request]) -> Result<(), Self::Error>;

    /// Syncs log file.
    fn sync(&self) -> Result<(), Self::Error>;
}

pub trait ValueLogGC: Sized + Send + Sync + 'static {
    type ValueLog: ValueLog<Error = Self::Error>;
    type Error: std::error::Error
        + From<<<Self::ValueLog as ValueLog>::Database as Database>::Error>;

    fn run(self, discard_ratio: f64) -> Result<(), Self::Error>;
}

pub trait Database: Sized + Send + Sync + 'static {
    type Error: std::error::Error
        + From<<Self::KeyRegistry as KeyRegistry>::Error>
        + Into<<Self::KeyRegistry as KeyRegistry>::Error>
        + From<<Self::MemoryTable as MemoryTable>::Error>
        + Into<<Self::MemoryTable as MemoryTable>::Error>
        + From<<Self::ValueLog as ValueLog>::Error>
        + Into<<Self::ValueLog as ValueLog>::Error>;

    type KeyRegistry: KeyRegistry;
    type ValueLog: ValueLog;
    type MemoryTable: MemoryTable;
    type Oracle: Oracle;

    /// Returns a new Database
    fn open(&self, path: &str) -> Result<Self, Self::Error>;

    fn max_version(&self) -> u64;

    /// Close closes a DB. It's crucial to call it to ensure all the pending updates make their way to
    /// disk. Calling DB.Close() multiple times would still only close the DB once.
    fn close(&self) -> Result<(), Self::Error>;

    /// Denotes if the badger DB is closed or not. A DB instance should not
    /// be used after closing it.
    fn is_closed(&self) -> bool;

    /// Verifies checksum for all tables on all levels.
    /// This method can be used to verify checksum, if opt.ChecksumVerificationMode is NoVerification.
    fn verify_checksum(&self) -> Result<(), Self::Error>;

    /// Syncs database content to disk. This function provides
    /// more control to user to sync data whenever required.
    fn sync(&self) -> Result<(), Self::Error>;

    fn handover_skiplist<F>(&self, skl: &skl::FixedSKL, callback: F) -> Result<(), Self::Error>
    where
        F: FnMut();

    fn new_skiplist(&self) -> skl::FixedSKL;

    /// Returns the value in memtable or disk for given key.
    /// Note that value will include meta byte.
    ///
    /// **IMPORTANT:** We should never write an entry with an older timestamp for the same key, We need to
    /// maintain this invariant to search for the latest value of a key, or else we need to search in all
    /// tables and find the max version among them.  To maintain this invariant, we also need to ensure
    /// that all versions of a key are always present in the same table from level 1, because compaction
    /// can push any table down.
    ///
    /// **Update(23/09/2020)** - We have dropped the move key implementation. Earlier we
    /// were inserting move keys to fix the invalid value pointers but we no longer
    /// do that. For every get("fooX") call where X is the version, we will search
    /// for "fooX" in all the levels of the LSM tree. This is expensive but it
    /// removes the overhead of handling move keys completely.
    fn get<Q>(&self, key: Q) -> Result<Value, Self::Error>
    where
        Q: core::borrow::Borrow<[u8]>;

    fn orc(&self) -> &Self::Oracle;

    fn registry(&self) -> &Self::KeyRegistry;

    fn open_mem_tables(&self, opts: MemTableOptions)
        -> Result<Vec<Self::MemoryTable>, Self::Error>;

    fn open_mem_table(
        &self,
        fid: u32,
        #[cfg(unix)] flags: i32,
        #[cfg(windows)] flags: u32,
    ) -> Result<Self::MemoryTable, Self::Error>;

    fn create_mem_table(&self) -> Result<Self::MemoryTable, Self::Error>;

    fn file_path(&self) -> Result<PathBuf, Self::Error>;

    fn value_threshold(&self) -> u64;

    fn update_threshold(&self, threshold: Vec<i64>);
}
