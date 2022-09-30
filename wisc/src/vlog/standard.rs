use super::{Error, Result, VALUE_LOG_FILE_EXTENSION};
use crate::{is_deleted_or_expired, Registry, WALOptions, VALUE_LOG_HEADER_SIZE, WAL};
use bable::kvstructs::{EntryRef, Key, KeyExt, Value, ValueExt, OP};
use core::sync::atomic::{AtomicI32, AtomicU32, Ordering};
use crossbeam_channel::{select, Receiver, Sender};
use fmmap::MetaDataExt;
use parking_lot::RwLock;
use std::{collections::HashMap, path::PathBuf};
use vela_traits::Database;
use vela_utils::ref_counter::RefCounter;

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

struct Inner {
    files_map: HashMap<u32, RefCounter<WAL>>,
    files_to_be_deleted: Vec<u32>,
    max_fid: u32,
    num_entries_written: u32,
    next_gc_fid: u32,
}

pub struct ValueLog<DB> {
    // guards our view of which files exist, which to be deleted, how many active iterators
    inner: RefCounter<RwLock<Inner>>,
    db: RefCounter<DB>,

    // A refcount of iterators -- when this hits zero, we can delete the filesToBeDeleted.
    num_active_iters: AtomicI32,

    writable_log_offset: AtomicU32,

    opts: ValueLogOptions,

    garbage_tx: Sender<()>,
    discard_stats: DiscardStats,
}

impl<DB: Database> ValueLog<DB> {
    pub fn open(db: RefCounter<DB>, opts: ValueLogOptions) -> Result<Option<Self>> {
        if opts.in_memory {
            return Ok(None);
        }

        let discard_stats = DiscardStats::new(&opts.dir_path)?;

        let (max_fid, files_map) = populate_files_map(&opts.dir_path, db.registry(), &opts)?;
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
            .collect::<HashMap<_, _>>();

        let next_gc_fid = RefCounter::new(AtomicU32::new(0));
        if opts.read_only {
            return Ok(Some(Self {
                discard_stats,
                garbage_tx: todo!(),
                opts,
                num_active_iters: AtomicI32::new(0),
                writable_log_offset: AtomicU32::new(0),
                inner: RefCounter::new(RwLock::new(Inner {
                    files_map,
                    files_to_be_deleted: Vec::new(),
                    max_fid,
                    num_entries_written: 0,
                    next_gc_fid: 0,
                })),
                db,
            }));
        }

        // Now we can read the latest value log file, and see if it needs truncation. We could
        // technically do this over all the value log files, but that would mean slowing down the value
        // log open.

        //     last, ok := vlog.filesMap[vlog.maxFid]
        // y.AssertTrue(ok)
        // lastOff, err := last.iterate(vlog.opt.ReadOnly, vlogHeaderSize,
        // 	func(_ Entry, vp valuePointer) error {
        // 		return nil
        // 	})
        // if err != nil {
        // 	return y.Wrapf(err, "while iterating over: %s", last.path)
        // }
        // if err := last.Truncate(int64(lastOff)); err != nil {
        // 	return y.Wrapf(err, "while truncating last value log file: %s", last.path)
        // }

        // // Don't write to the old log file. Always create a new one.
        // if _, err := vlog.createVlogFile(); err != nil {
        // 	return y.Wrapf(err, "Error while creating log file in valueLog.open")
        // }
        Ok(Some(Self {
            opts,
            discard_stats,
            garbage_tx: todo!(),
            num_active_iters: todo!(),
            writable_log_offset: todo!(),
            inner: RefCounter::new(RwLock::new(Inner {
                files_map,
                files_to_be_deleted: Vec::new(),
                max_fid,
                num_entries_written: 0,
                next_gc_fid: 0,
            })),
            db,
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
) -> Result<(u32, HashMap<u32, RefCounter<WAL>>)> {
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
                                        map.insert(fid, RefCounter::new(wal));
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
fn discard_entry(e: EntryRef, v: Value) -> bool {
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

pub struct ValueLogGC<DB> {
    inner: RefCounter<RwLock<Inner>>,
    discard_stats: DiscardStats,
    next_gc_fid: RefCounter<AtomicU32>,
    garbage_tx: Sender<()>,
    garbage_rx: Receiver<()>,
    db: RefCounter<DB>,
}

impl<DB: Database> ValueLogGC<DB> {
    pub fn run(self, discard_ratio: f64) -> Result<()> {
        select! {
            send(self.garbage_tx, ()) -> _ => {
                // Pick a log file for GC.
                let lf = self.pick_log(discard_ratio)?;
                if let Some(lf) = lf {
                    return self.do_run_gc(&lf);
                }
                Err(Error::NoRewrite)
            }
            default => {
                Err(Error::Rejected)
            }
        }
    }

    pub fn pick_log(&self, discard_ratio: f64) -> Result<Option<RefCounter<WAL>>> {
        let inner = self.inner.read();
        'outer: loop {
            // Pick a candidate that contains the largest amount of discardable data
            let (fid, discard) = self.discard_stats.max_discard();

            // MaxDiscard will return fid=0 if it doesn't have any discard data. The
            // vlog files start from 1.
            if fid == 0 {
                let next_gc_fid = self.next_gc_fid.load(Ordering::Acquire);
                for fid in next_gc_fid..inner.max_fid {
                    if let Some(wal) = inner.files_map.get(&fid) {
                        let discarded = self.calculate_discard_stat(
                            wal,
                            &inner.files_to_be_deleted,
                            inner.max_fid,
                        );
                        match discarded {
                            Ok(discarded) => {
                                if discarded < discard_ratio {
                                    continue;
                                }
                                self.next_gc_fid.store(fid + 1, Ordering::Release);
                                return Ok(Some(wal.clone()));
                            }
                            Err(_) => continue,
                        }
                    } else {
                        continue;
                    }
                }

                // reset the counter so next time we will start from the start
                self.next_gc_fid.store(0, Ordering::Release);

                return Ok(None);
            }

            match inner.files_map.get(&fid) {
                Some(lf) => {
                    // We have a valid file.
                    return lf
                        .stat()
                        .map(|stat| {
                            let thr = discard_ratio * stat.size() as f64;
                            if (discard as f64) < thr {
                                // TODO: filename in metadata
                                #[cfg(feature = "tracing")]
                                {
                                    tracing::debug!(
                                        target: "vlog_gc",
                                        "Discard: {} less than threshold: {} for file",
                                        discard,
                                        thr
                                    );
                                }
                                return None;
                            }

                            let max_fid = inner.max_fid;
                            if fid < max_fid {
                                #[cfg(feature = "tracing")]
                                {
                                    tracing::info!(
                                        target: "vlog_gc",
                                        "Found value log max discard fid: {} discard: {}",
                                        fid,
                                        discard
                                    );
                                }
                                return Some(lf.clone());
                            }
                            None
                        })
                        .map_err(|e| {
                            #[cfg(feature = "tracing")]
                            {
                                tracing::error!(
                                    target: "vlog_gc",
                                    err = %e,
                                    "Unable to get stats for value log fid: {}",
                                    fid
                                );
                            }
                            e
                        });
                }
                // This file was deleted but it's discard stats increased because of compactions. The file
                // doesn't exist so we don't need to do anything. Skip it and retry.
                None => {
                    self.discard_stats.update(fid, -1);
                    continue 'outer;
                }
            }
        }
    }

    /// Returns discard ratio for the specified wal.
    fn calculate_discard_stat(&self, f: &WAL, to_be_deleted: &[u32], max_fid: u32) -> Result<f64> {
        for fid in to_be_deleted {
            if fid.eq(&f.fid) {
                return Err(Error::AlreadyMarkedDeletion(*fid));
            }
        }

        assert!(
            f.fid < max_fid,
            "fid to calculate_discard_stat: {}. Current max fid: {}",
            f.fid,
            max_fid
        );

        let mut count = 0;
        let mut discarded = 0;
        let mut fe = |ent: EntryRef| {
            count += 1;
            let key = ent.get_key().parse_key();
            let ts = self.db.orc().read_timestamp();
            self.db
                .get(Key::copy_from_slice(key).with_timestamp(ts))
                .map(|vs| {
                    if discard_entry(ent, vs) {
                        discarded += 1;
                    }
                })
                .map_err(Into::into)
        };

        f.iter(0, |ent, vp| fe(ent))
            .map(|_| (discarded as f64) / (count as f64))
    }

    #[inline]
    fn rewrite(&self, f: &WAL) -> Result<()> {
        // TODO:
        unimplemented!()
    }

    #[inline]
    fn do_run_gc(&self, lf: &WAL) -> Result<()> {
        //TODO: self.rewrite

        // Remove the file from discardStats.
        self.discard_stats.update(lf.fid, -1);
        Ok(())
    }
}
