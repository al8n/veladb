use super::{Result, VALUE_LOG_FILE_EXTENSION};
use crate::{is_deleted_or_expired, Registry, WALOptions, VALUE_LOG_HEADER_SIZE, WAL};
use bable::kvstructs::{Entry, KeyExt, Value, ValueExt, OP};
use core::sync::atomic::{AtomicI32, AtomicU32, Ordering};
use crossbeam_channel::{Receiver, Sender};
use std::{collections::HashMap, path::PathBuf};

mod discard;
use discard::*;

mod threshold;
use threshold::*;

pub struct ValueLogOptions {
    pub size: u64,
    pub in_memory: bool,
    pub read_only: bool,
    pub sync_writes: bool,
    pub enable_metrics: bool,
    pub dir_path: PathBuf,
    pub file_size: u64,
    pub save_on_drop: bool,
}

impl<'a> From<&'a ValueLogOptions> for WALOptions {
    fn from(opts: &'a ValueLogOptions) -> Self {
        WALOptions {
            sync_writes: opts.sync_writes,
            read_only: opts.read_only,
            enable_metrics: opts.enable_metrics,
            mem_table_size: 2 * opts.file_size,
            save_on_drop: opts.save_on_drop,
        }
    }
}

pub struct ValueLog {
    // guards our view of which files exist, which to be deleted, how many active iterators
    files_map: HashMap<u32, WAL>,

    max_fid: u32,
    next_gc_fid: u32,
    files_to_be_deleted: Vec<u32>,

    // A refcount of iterators -- when this hits zero, we can delete the filesToBeDeleted.
    num_active_iters: AtomicI32,

    writable_log_offset: AtomicU32,
    num_entries_written: u32,
    opts: ValueLogOptions,

    garbage_tx: Sender<()>,
    discard_stats: DiscardStats,
}

impl ValueLog {
    pub fn open(registry: &Registry, opts: ValueLogOptions) -> Result<Option<Self>> {
        if opts.in_memory {
            return Ok(None);
        }

        let discard_stats = DiscardStats::new(&opts.dir_path)?;

        let (max_fid, files_map) = populate_files_map(&opts.dir_path, registry, &opts)?;
        let files_map = files_map
            .into_iter()
            .filter(|(fid, wal)| {

                // We shouldn't delete the maxFid file.
                if wal.size.load(Ordering::Relaxed) == VALUE_LOG_HEADER_SIZE as u32 && fid.ne(&max_fid) {
                    // delete empty files
                    #[cfg(feature = "tracing")]
                    {
                        tracing::info!(target: "value_log", "deleting empty file: {}", wal.path.display());
                    }
                    false
                } else {
                    true
                }
            })
            .collect::<HashMap<u32, WAL>>();

        if opts.read_only {
            return Ok(Some(Self {
                max_fid,
                discard_stats,
                garbage_tx: Sender::default(),
                opts,
                files_map,
                next_gc_fid: 0,
                files_to_be_deleted: Vec::new(),
                num_active_iters: AtomicI32::new(0),
                writable_log_offset: AtomicU32::new(0),
                num_entries_written: 0,
            }));
        }

        Ok(Some(Self {
            max_fid: 0,
            opts,
            discard_stats,
            files_map,
            garbage_tx: todo!(),
            next_gc_fid: todo!(),
            files_to_be_deleted: todo!(),
            num_active_iters: todo!(),
            writable_log_offset: todo!(),
            num_entries_written: todo!(),
        }))
    }

    #[inline]
    fn fpath(&self, fid: u32) -> PathBuf {
        Self::file_path(&self.opts.dir_path, fid)
    }

    #[inline]
    fn file_path(dir_path: &PathBuf, max_fid: u32) -> PathBuf {
        let mut path = dir_path.join(format!("{:06}", max_fid));
        path.set_extension(VALUE_LOG_FILE_EXTENSION);
        path
    }
}

fn populate_files_map(
    dir: &PathBuf,
    registry: &Registry,
    opts: &ValueLogOptions,
) -> Result<(u32, HashMap<u32, WAL>)> {
    std::fs::read_dir(dir).map_err(From::from).and_then(|dir| {
        let mut map = HashMap::new();
        let mut max_fid = 0;
        for entry in dir {
            let entry = entry?;
            let metadata = entry.metadata()?;
            if metadata.is_file() {
                let path = entry.path();
                if let Some(ext) = path.extension() {
                    if ext == VALUE_LOG_FILE_EXTENSION {
                        if let Some(filename) = path.file_stem() {
                            if let Some(filename) = filename.to_str() {
                                if let Ok(fid) = filename.parse::<u32>() {
                                    // Just open in RDWR mode. This should not create a new log file.
                                    #[cfg(unix)]
                                    let flags = rustix::fs::OFlags::RDWR.bits();
                                    #[cfg(windows)]
                                    let flags = rustix::fs::OFlags::RDWR.bits() as u32;

                                    let wal = WAL::open(
                                        path,
                                        flags,
                                        fid,
                                        registry.clone(),
                                        WALOptions::from(opts),
                                    )?;
                                    if let Some(wal) = wal {
                                        if max_fid < fid {
                                            max_fid = fid;
                                        }
                                        map.insert(fid, wal);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok((max_fid, map))
    })
}

#[inline]
fn discard_entry(e: Entry, v: Value) -> bool {
    let meta = v.get_meta();
    let version = v.get_version();
    if version != e.get_key().parse_timestamp() {
        // Version not found. Discard.
        return true;
    }

    let expires_at = v.get_expires_at();
    if is_deleted_or_expired(meta, expires_at) {
        return true;
    }

    if (meta & OP::BIT_VALUE_POINTER.bits()) == 0 {
        // Key also stores the value in LSM. Discard.
        return true;
    }

    if (meta & OP::BIT_FIN_TXN.bits()) > 0 {
        // Just a txn finish entry. Discard.
        return true;
    }

    false
}

struct ValueLogGC {
    discard_stats: DiscardStats,
}
