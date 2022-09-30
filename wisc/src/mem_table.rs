use core::sync::atomic::{AtomicU64, Ordering};

use crate::{error::*, WAL};
use vela_traits::{LogFile, MemoryTable};
use vpb::kvstructs::{
    bytes::BytesMut, Entry, EntryRef, Key, KeyExt, Value, ValueExt, ValuePointer, OP,
};

const MEM_FILE_EXTENSION: &str = "mem";

pub struct MemTableOptions {
    pub size: u64,
    pub in_memory: bool,
    pub read_only: bool,
}

/// Stores a skiplist and a corresponding WAL. Writes to memTable are written
/// both to the WAL and the skiplist. On a crash, the WAL is replayed to bring the skiplist back to
/// its pre-crash form.
pub struct MemTable {
    skl: skl::FixedSKL,
    wal: Option<WAL>,
    max_version: AtomicU64,
    buf: BytesMut,
    opts: MemTableOptions,
}

impl MemoryTable for MemTable {
    type Error = Error;

    #[inline]
    fn sync_wal(&self) -> Result<()> {
        if let Some(wal) = &self.wal {
            wal.sync()
        } else {
            Ok(())
        }
    }

    #[inline]
    fn is_full(&self) -> bool {
        if self.skl.mem_size() as u64 >= self.opts.size {
            return true;
        }

        if self.opts.in_memory {
            // InMemory mode doesn't have any WAL.
            return false;
        }

        self.wal.as_ref().unwrap().write_at.load(Ordering::Relaxed) as u64 >= self.opts.size
    }

    fn insert(&mut self, key: Key, val: Value) -> Result<()> {
        let ent = Entry::new_from_kv(key, val);

        // wal is nil only when veladb in running in in-memory mode and we don't need the wal.
        if let Some(wal) = &self.wal {
            wal.write_entry(&mut self.buf, &ent).map(|_| {
                // We insert the finish marker in the WAL but not in the memtable.
                if ent.val.get_meta() & OP::BIT_FIN_TXN.bits() == 0 {
                    let ts = ent.key.parse_timestamp();
                    self.skl.insert(ent.key, ent.val);
                    let max_version = self.max_version.load(Ordering::Acquire);
                    if ts > max_version {
                        self.max_version.store(ts, Ordering::Release);
                    }
                }
            })
        } else {
            // We insert the finish marker in the WAL but not in the memtable.
            if ent.val.get_meta() & OP::BIT_FIN_TXN.bits() > 0 {
                return Ok(());
            }

            let ts = ent.key.parse_timestamp();
            self.skl.insert(ent.key, ent.val);
            let max_version = self.max_version.load(Ordering::Acquire);
            if ts > max_version {
                self.max_version.store(ts, Ordering::Release);
            }
            Ok(())
        }
    }

    fn update_skiplist(&self) -> Result<()> {
        if let Some(wal) = &self.wal {
            let mut first = true;
            let replay = |ent: EntryRef, _vp: ValuePointer| -> Result<()> {
                #[cfg(feature = "tracing")]
                if first {
                    tracing::debug!(target: "memtable", "First key={:?}", ent.key.as_slice());
                }
                first = false;
                let ts = ent.key.parse_timestamp();
                let max_version = self.max_version.load(Ordering::Acquire);
                if ts > max_version {
                    self.max_version.store(ts, Ordering::Release);
                }

                // This is already encoded correctly. Value would be either a vptr, or a full value
                // depending upon how big the original value was. Skiplist makes a copy of the key and
                // value.
                self.skl.insert(ent.key, ent.val);
                Ok(())
            };

            wal.iter(0, replay).and_then(|end_offset| {
                let wal_size = wal.size.load(Ordering::Relaxed);
                if end_offset < wal_size && self.opts.read_only {
                    return Err(Error::TruncateNeeded {
                        end_offset,
                        size: wal_size,
                    });
                }
                wal.truncate(end_offset as u64)
            })
        } else {
            Ok(())
        }
    }
}
