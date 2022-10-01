use super::{Error, Result, MAX_VALUE_LOG_SIZE, VALUE_LOG_FILE_EXTENSION};
use crate::{is_deleted_or_expired, Registry, VALUE_LOG_HEADER_SIZE, WAL};
use core::sync::atomic::{AtomicI32, AtomicU32, Ordering};
use crossbeam_channel::{select, Receiver, Sender};
use fmmap::MetaDataExt;
use parking_lot::RwLock;
use std::{collections::HashMap, path::PathBuf};
use vela_options::{LogFileOptions, ValueLogOptions};
use vela_traits::{Database, LogFile, Oracle, ValueLog, ValueLogGC};
use vela_utils::ref_counter::RefCounter;
use vpb::{
    checksum::crc32fast,
    kvstructs::{
        bytes::{Bytes, BytesMut},
        request::Request,
        EntryRef, Header, Key, KeyExt, Value, ValueExt, ValuePointer, OP,
    },
};

mod discard;
use discard::*;

mod threshold;
use threshold::*;

struct Inner {
    files_map: HashMap<u32, RefCounter<WAL>>,
    files_to_be_deleted: Vec<u32>,
    max_fid: u32,
    next_gc_fid: u32,
}

pub struct WiscValueLog<DB> {
    // guards our view of which files exist, which to be deleted, how many active iterators
    inner: RefCounter<RwLock<Inner>>,
    db: RefCounter<DB>,

    // A refcount of iterators -- when this hits zero, we can delete the filesToBeDeleted.
    num_active_iters: AtomicI32,
    num_entries_written: AtomicU32,

    writable_log_offset: AtomicU32,

    opts: ValueLogOptions,

    garbage_tx: Sender<()>,
    discard_stats: DiscardStats,
}

impl<DB> WiscValueLog<DB> {
    #[inline]
    fn fpath(&self, fid: u32) -> PathBuf {
        Self::file_path(self.opts.dir_path(), fid)
    }

    #[inline]
    fn file_path(dir_path: &PathBuf, max_fid: u32) -> PathBuf {
        let mut path = dir_path.join(format!("{:06}", max_fid));
        path.set_extension(VALUE_LOG_FILE_EXTENSION);
        path
    }
}

impl<DB: Database<KeyRegistry = Registry>> WiscValueLog<DB>
where
    Error: From<<DB as Database>::Error>,
{
    #[allow(clippy::type_complexity)]
    fn populate_files_map<P: AsRef<std::path::Path>>(
        dir: P,
        registry: &<<Self as ValueLog>::LogFile as LogFile>::KeyRegistry,
        opts: &ValueLogOptions,
    ) -> core::result::Result<
        (u32, HashMap<u32, RefCounter<<Self as ValueLog>::LogFile>>),
        <Self as ValueLog>::Error,
    > {
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

                                        <WAL as LogFile>::open(
                                            path,
                                            flags,
                                            fid,
                                            registry.clone(),
                                            LogFileOptions::from(opts),
                                        )
                                        .map(|wal| {
                                            if max_fid < fid {
                                                max_fid = fid;
                                            }
                                            HashMap::insert(&mut map, fid, RefCounter::new(wal))
                                        })?;
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

    /// Check whether the given requests can fit into 4GB vlog file.
    ///
    /// **NOTE:** 4GB is the maximum size we can create for vlog because value pointer offset is of type
    /// `u32`. If we create more than 4GB, it will overflow `u32`. So, limiting the size to 4GB.
    #[inline]
    fn validate_writes(&self, reqs: &mut [Request]) -> Result<()> {
        let mut vlog_offset = self.writable_log_offset.load(Ordering::SeqCst) as u64;
        for req in reqs {
            // calculate size of the request.
            let size = req.estimated_size();
            let estimated_vlog_offset = vlog_offset + size;
            if estimated_vlog_offset > MAX_VALUE_LOG_SIZE as u64 {
                return Err(Error::MaxValueLogSize {
                    estimated_offset: estimated_vlog_offset,
                    max_value_log_size: MAX_VALUE_LOG_SIZE,
                });
            }

            if estimated_vlog_offset >= self.opts.file_size() {
                // We'll create a new vlog file if the estimated offset is greater or equal to
                // max vlog size. So, resetting the vlogOffset.
                vlog_offset = 0;
                continue;
            }

            // Estimated vlog offset will become current vlog offset if the vlog is not rotated.
            vlog_offset = estimated_vlog_offset;
        }
        Ok(())
    }
}

impl<DB: Database<KeyRegistry = Registry>> ValueLog for WiscValueLog<DB>
where
    Error: From<<DB as Database>::Error>,
{
    type Database = DB;

    type GC = WiscValueLogGC<Self::Database>;

    type LogFile = WAL;

    type Error = Error;

    fn open(
        db: RefCounter<Self::Database>,
        opts: ValueLogOptions,
    ) -> core::result::Result<Option<Self>, Self::Error> {
        if opts.in_memory() {
            return Ok(None);
        }

        let discard_stats = DiscardStats::new(opts.dir_path())?;

        let (max_fid, files_map) = Self::populate_files_map(opts.dir_path(), db.registry(), &opts)?;
        let mut files_map = files_map
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

        let next_gc_fid = 0;
        if opts.read_only() {
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
                    next_gc_fid,
                })),
                num_entries_written: AtomicU32::new(0),
                db,
            }));
        }

        // Now we can read the latest value log file, and see if it needs truncation. We could
        // technically do this over all the value log files, but that would mean slowing down the value
        // log open.
        let last = files_map.get(&max_fid).unwrap();
        last.iter(VALUE_LOG_HEADER_SIZE as u32, |_, _| Ok(()))
            .map_err(|e| {
                #[cfg(feature = "tracing")]
                {
                    tracing::error!(target: "value_log", err = %e, "while iterating over: {}", last.path.display());
                }
                e
            })
            .and_then(|last_off| {
                last.truncate(last_off as u64).map_err(|e| {
                    #[cfg(feature = "tracing")]
                    {
                        tracing::error!(target: "value_log", err = %e, "while truncating last value log file: {}", last.path.display());
                    }
                    e
                })
            })?;

        // Don't write to the old log file. Always create a new one.
        Self::create(opts.dir_path(), db.registry().clone(), max_fid, LogFileOptions::from(&opts))
            .map(|lf| {
                files_map.insert(max_fid, RefCounter::new(lf));

                // writableLogOffset is only written by write func, by read by Read func.
                // To avoid a race condition, all reads and updates to this variable must be
                // done via atomics.
                Some(Self {
                    opts,
                    discard_stats,
                    garbage_tx: todo!(),
                    num_entries_written: AtomicU32::new(0),
                    num_active_iters: AtomicI32::new(0),
                    writable_log_offset: AtomicU32::new(VALUE_LOG_HEADER_SIZE as u32),
                    inner: RefCounter::new(RwLock::new(Inner {
                        files_map,
                        files_to_be_deleted: Vec::new(),
                        max_fid,
                        next_gc_fid,
                    })),
                    db,
                })
            })
            .map_err(|e| {
                #[cfg(feature = "tracing")]
                {
                    tracing::error!(target: "value_log", err = %e, "while creating new value log file in WiscValueLog::open");
                }
                e
            })
    }

    fn create(
        dir_path: &PathBuf,
        registry: <Self::LogFile as LogFile>::KeyRegistry,
        max_fid: u32,
        opts: LogFileOptions,
    ) -> core::result::Result<Self::LogFile, Self::Error> {
        let path = Self::file_path(dir_path, max_fid);
        let flags = {
            #[cfg(unix)]
            {
                (rustix::fs::OFlags::RDWR | rustix::fs::OFlags::CREATE | rustix::fs::OFlags::EXCL)
                    .bits()
            }

            #[cfg(windows)]
            {
                (rustix::fs::OFlags::RDWR | rustix::fs::OFlags::CREATE | rustix::fs::OFlags::EXCL)
                    .bits() as u32
            }
        };

        <WAL as LogFile>::open(path, flags, max_fid, registry, opts)
    }

    fn read(&self, vp: ValuePointer) -> Result<Bytes> {
        let inner = RwLock::read(&self.inner);
        match inner.files_map.get(&vp.fid) {
            Some(lf) => {
                // Check for valid offset if we are reading from writable log.
                let max_fid = inner.max_fid;
                // In read-only mode we don't need to check for writable offset as we are not writing anything.
                // Moreover, this offset is not set in readonly mode.
                if !self.opts.read_only() && vp.fid == max_fid {
                    let current_offset = self.writable_log_offset.load(Ordering::SeqCst);
                    if vp.offset >= current_offset {
                        return Err(Error::InvalidValuePointer {
                            current: current_offset,
                            pointer: vp.offset,
                        });
                    }
                }

                let guard = WAL::read(lf, vp)?;
                let buf = guard.data();
                let buf_size = buf.len() - core::mem::size_of::<u32>();
                if self.opts.verify_value_checksum() {
                    let mut hash = crc32fast::Hasher::new();

                    hash.update(&buf[..buf_size]);
                    // Fetch checksum from the end of the buffer.
                    let checksum = u32::from_be_bytes(buf[buf_size..].try_into().unwrap());
                    let buf_cks = hash.finalize();
                    if buf_cks != checksum {
                        return Err(Error::ChecksumMismatch {
                            expected: checksum,
                            actual: buf_cks,
                        });
                    }
                }
                let (header_len, header) = Header::decode(buf);
                let kv: Bytes = WAL::decrypt_kv(
                    &buf[header_len..],
                    lf.secret(),
                    lf.base_iv(),
                    vp.offset,
                    lf.encryption_algorithm(),
                )?
                .into();
                let klen = header.get_key_len() as usize;
                let vlen = header.get_value_len() as usize;
                if kv.len() < klen + vlen {
                    #[cfg(feature = "tracing")]
                    {
                        tracing::error!(target: "value_log", "invalid read vp: {:?}", vp);
                    }
                    return Err(Error::InvalidRead {
                        len: kv.len(),
                        range: klen..klen + vlen,
                    });
                }

                Ok(kv.slice(klen..klen + vlen))
            }
            None => Err(Error::LogFileNotFound(vp.fid)),
        }
    }

    fn write(&self, reqs: &mut [Request]) -> Result<()> {
        if self.opts.in_memory() || self.opts.managed_txns() {
            // Don't do value log writes in managed mode.
            return Ok(());
        }

        // Validate writes before writing to vlog. Because, we don't want to partially write and return
        // an error.
        self.validate_writes(reqs).map_err(|e| {
            #[cfg(feature = "tracing")]
            {
                tracing::error!(target: "value_log", err = %e, "while validating writes");
            }
            e
        })?;

        let inner = RwLock::read(&self.inner);

        let max_fid = inner.max_fid;
        let mut cur_lf = inner.files_map.get(&max_fid).unwrap().clone();
        drop(inner);

        let sync = |lf: &WAL| {
            if self.opts.sync_writes() {
                if let Err(e) = lf.sync() {
                    #[cfg(feature = "tracing")]
                    {
                        tracing::error!(target: "value_log", err = %e, "while syncing current value log file");
                    }
                }
            }
        };

        let write = |buf: &mut BytesMut, lf: &WAL| {
            if buf.is_empty() {
                return Ok(());
            }

            let n = buf.len() as u32;
            let end_offset = self.writable_log_offset.fetch_add(n, Ordering::SeqCst) + n;
            // Increase the file size if we cannot accommodate this entry.
            lf.read_from_buffer(buf, n, end_offset)
        };

        let to_disk = |lf: &WAL| {
            let num_entries_written = self.num_entries_written.load(Ordering::SeqCst);
            if self.writable_log_offset.load(Ordering::SeqCst) > (self.opts.file_size() as u32)
                || num_entries_written > self.opts.max_entries()
            {
                return lf
                    .done_writing(self.writable_log_offset.load(Ordering::SeqCst))
                    .map(|_| true);
            }

            Ok(false)
        };

        let mut buf = BytesMut::new();
        for req in reqs {
            req.ptrs.clear();
            let mut written = 0;
            let mut written_bytes = 0;
            let mut value_sizes = Vec::with_capacity(req.entries.len());
            for j in 0..req.entries.len() {
                buf.clear();

                let entry = &mut req.entries[j];
                value_sizes.push(entry.val.len() as i64);
                if entry.skip_vlog_and_set_threshold(self.db.value_threshold()) {
                    req.ptrs.push(ValuePointer::default());
                    continue;
                }

                let mut vp = ValuePointer {
                    fid: cur_lf.fid,
                    len: 0,
                    offset: self.writable_log_offset.load(Ordering::SeqCst),
                };

                // We should not store transaction marks in the vlog file because it will never have all
                // the entries in a transaction. If we store entries with transaction marks then value
                // GC will not be able to iterate on the entire vlog file.
                // But, we still want the entry to stay intact for the memTable WAL. So, store the meta
                // in a temporary variable and reassign it after writing to the value log.
                let tmp_meta = entry.val.get_meta();
                entry.val.meta = tmp_meta & !(OP::BIT_TXN | OP::BIT_FIN_TXN).bits();

                match cur_lf.encode_entry(&mut buf, entry, vp.offset) {
                    Ok(plen) => {
                        // Restore the meta.
                        entry.val.meta = tmp_meta;
                        vp.len = plen as u32;
                        req.ptrs.push(vp);

                        write(&mut buf, &cur_lf)
                            .map(|_| {
                                written += 1;
                                written_bytes += buf.len();
                            })
                            .map_err(|e| {
                                sync(&cur_lf);
                                e
                            })?;
                        // No need to flush anything, we write to file directly via mmap.
                    }
                    Err(e) => {
                        #[cfg(feature = "tracing")]
                        {
                            tracing::error!(target: "value_log", err = %e, "while encoding entry");
                        }
                        sync(&cur_lf);
                        return Err(e);
                    }
                }
            }

            self.num_entries_written
                .fetch_add(written, Ordering::SeqCst);
            // We write to disk here so that all entries that are part of the same transaction are
            // written to the same vlog file.
            self.db.update_threshold(value_sizes);

            // We write to disk here so that all entries that are part of the same transaction are
            // written to the same vlog file.
            let create_new = to_disk(&cur_lf).map_err(|e| {
                sync(&cur_lf);
                e
            })?;
            if create_new {
                cur_lf = self.append().map_err(|e| {
                    sync(&cur_lf);
                    e
                })?;
            }
        }

        if to_disk(&cur_lf)? {
            return match self.append() {
                Ok(_) => Ok(()),
                Err(e) => {
                    sync(&cur_lf);
                    Err(e)
                }
            };
        }
        sync(&cur_lf);
        Ok(())
    }

    /// Syncs content of latest value log file to disk. Syncing of value log directory is
    /// not required here as it happens every time a value log file rotation happens(check [`WiscValueLog::create`]
    /// function). During rotation, previous value log file also gets synced to disk. It only syncs file
    /// if `fid >= self.max_fid()`. In some cases such as replay(while opening db), it might be called with
    /// `fid < vlog.max_fid()`. To sync irrespective of file id just call it with `u32::MAX`.
    fn sync(&self) -> Result<()> {
        if self.opts.sync_writes() || self.opts.in_memory() {
            return Ok(());
        }

        let inner = RwLock::read(&self.inner);
        let max_fid = inner.max_fid;

        match inner.files_map.get(&max_fid) {
            Some(lf) => WAL::sync(lf),
            // Sometimes it is possible that max_fid has been increased but file creation
            // with same id is still in progress and this function is called. In those cases
            // entry for the file might not be present in files_map.
            None => Ok(()),
        }
    }

    fn append(&self) -> Result<RefCounter<Self::LogFile>> {
        let mut inner = RwLock::write(&self.inner);
        let fid = inner.max_fid + 1;
        let path = self.fpath(fid);
        let flags = {
            #[cfg(unix)]
            {
                (rustix::fs::OFlags::RDWR | rustix::fs::OFlags::CREATE | rustix::fs::OFlags::EXCL)
                    .bits()
            }

            #[cfg(windows)]
            {
                (rustix::fs::OFlags::RDWR | rustix::fs::OFlags::CREATE | rustix::fs::OFlags::EXCL)
                    .bits() as u32
            }
        };

        <WAL as LogFile>::open(
            path,
            flags,
            fid,
            self.db.registry().clone(),
            LogFileOptions::from(&self.opts),
        )
        .map(|lf| {
            let lf = RefCounter::new(lf);
            inner.files_map.insert(fid, lf.clone());
            inner.max_fid = fid;

            // writableLogOffset is only written by write func, by read by Read func.
            // To avoid a race condition, all reads and updates to this variable must be
            // done via atomics.
            self.writable_log_offset
                .store(VALUE_LOG_HEADER_SIZE as u32, Ordering::SeqCst);
            self.num_entries_written.store(0, Ordering::SeqCst);
            lf
        })
        .map_err(From::from)
    }
}

impl<DB> Drop for WiscValueLog<DB> {
    fn drop(&mut self) {
        if self.opts.in_memory() {
            return;
        }

        let inner = self.inner.read();
        for (id, lf) in inner.files_map.iter() {
            if !self.opts.read_only() && id == &inner.max_fid {
                let woffset = self.writable_log_offset.load(Ordering::SeqCst);
                lf.truncate(woffset as u64).unwrap();
            }
        }
    }
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

pub struct WiscValueLogGC<DB> {
    inner: RefCounter<RwLock<Inner>>,
    discard_stats: DiscardStats,
    next_gc_fid: RefCounter<AtomicU32>,
    garbage_tx: Sender<()>,
    garbage_rx: Receiver<()>,
    db: RefCounter<DB>,
}

impl<DB: Database<KeyRegistry = Registry>> ValueLogGC for WiscValueLogGC<DB>
where
    Error: From<<DB as Database>::Error>,
{
    type ValueLog = WiscValueLog<DB>;

    type Error = Error;

    fn run(self, discard_ratio: f64) -> Result<()> {
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
}

impl<DB: Database<KeyRegistry = Registry>> WiscValueLogGC<DB>
where
    Error: From<<DB as Database>::Error>,
{
    fn pick_log(&self, discard_ratio: f64) -> Result<Option<RefCounter<WAL>>> {
        let inner = RwLock::read(&self.inner);
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
                .map_err(From::from)
        };

        f.iter(0, |ent, _vp| fe(ent))
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
