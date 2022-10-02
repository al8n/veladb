use crate::{error::*, registry::Registry, SafeRead, VALUE_LOG_HEADER_SIZE};
use core::{
    cell::Cell,
    sync::atomic::{AtomicU32, Ordering},
};
use fmmap::{MetaDataExt, MmapFileExt, MmapFileMut, MmapFileMutExt, Options};
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use rand::{thread_rng, RngCore};
use std::path::{Path, PathBuf};
use vela_options::LogFileOptions;
use vela_traits::{KeyRegistry, LogFile, ValueGuard};
use vpb::{
    encrypt::{Encryptor, BLOCK_SIZE},
    kvstructs::{
        bytes::{BufMut, Bytes, BytesMut},
        Entry, EntryRef, Header, Key, Value, ValueExt, ValuePointer, MAX_HEADER_SIZE, OP,
    },
    DataKey, EncryptionAlgorithm,
};

const WAL_IV_SIZE: usize = 12;

/// WAL is lock-free
///
/// Structure of WAL:
///
/// ```text
/// +----------------+------------------+------------------+
/// | keyID(8 bytes) | baseIV(12 bytes) |     entry...     |
/// +----------------+------------------+------------------+
/// ```
pub struct WAL {
    inner: RwLock<MmapFileMut>,
    pub(crate) path: PathBuf,
    pub(crate) fid: u32,
    pub(crate) write_at: AtomicU32,
    pub(crate) size: AtomicU32,
    data_key: Option<DataKey>,
    base_iv: [u8; WAL_IV_SIZE],
    registry: Registry,
    opt: LogFileOptions,
}

/// Safety: the `write_at` is only accessed when inner lock is hold.
unsafe impl Sync for WAL {}
/// Safety: the `write_at` is only accessed when inner lock is hold.
unsafe impl Send for WAL {}

#[allow(clippy::declare_interior_mutable_const)]
const EMPTY_BYTES: Value = Value::new();

pub struct ReadGuard<'a> {
    start: usize,
    end: usize,
    pub(crate) val: Value,
    inner: RwLockReadGuard<'a, MmapFileMut>,
}

impl<'a> ValueExt for ReadGuard<'a> {
    fn parse_value(&self) -> &[u8] {
        self.val.parse_value()
    }

    fn parse_value_to_bytes(&self) -> Bytes {
        self.val.parse_value_to_bytes()
    }

    fn get_meta(&self) -> u8 {
        self.val.get_meta()
    }

    fn get_user_meta(&self) -> u8 {
        self.val.get_user_meta()
    }

    fn get_expires_at(&self) -> u64 {
        self.val.get_expires_at()
    }
}

impl<'a> ValueGuard for ReadGuard<'a> {
    fn into_value(self) -> Value {
        self.val
    }
}

impl<'a> core::ops::Deref for ReadGuard<'a> {
    type Target = Value;

    fn deref(&self) -> &Self::Target {
        &self.val
    }
}

impl<'a> core::ops::DerefMut for ReadGuard<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.val
    }
}

impl<'a> ReadGuard<'a> {
    pub(crate) fn data(&self) -> &[u8] {
        &self.inner.as_slice()[self.start..self.end]
    }

    #[inline]
    pub fn into_value(self) -> Value {
        self.val
    }
}

impl LogFile for WAL {
    type Error = Error;

    type KeyRegistry = Registry;

    fn open<P: AsRef<std::path::Path>>(
        path: P,
        #[cfg(unix)] flags: i32,
        #[cfg(windows)] flags: u32,
        fid: u32,
        registry: Registry,
        opts: LogFileOptions,
    ) -> Result<Self> {
        let save = opts.save_on_drop;
        if !path.as_ref().exists() {
            Options::new()
                .max_size(opts.mem_table_size * 2)
                .custom_flags(flags)
                .create_mmap_file_mut(&path)
                .map_err(From::from)
                .and_then(|mut mmap| {
                    if !save {
                        mmap.set_remove_on_drop(true);
                    }
                    Self::bootstrap(path.as_ref().to_path_buf(), fid, registry, mmap, opts)
                })
        } else {
            Options::new()
                .max_size(opts.mem_table_size * 2)
                .custom_flags(flags)
                .create(true)
                .write(true)
                .read(true)
                .open_mmap_file_mut(&path)
                .map_err(From::from)
                .and_then(|mut mmap| {
                    if !save {
                        mmap.set_remove_on_drop(true);
                    }
                    let size = mmap.len();
                    Self::open_in(path.as_ref().to_path_buf(), fid, mmap, registry, size, opts)
                })
        }
    }

    #[inline]
    fn sync(&self) -> Result<()> {
        self.inner.read().flush().map_err(From::from)
    }

    #[inline]
    fn stat(&self) -> Result<fmmap::MetaData> {
        self.inner.read().metadata().map_err(From::from)
    }

    fn truncate(&self, end: u64) -> Result<()> {
        let mut inner = self.inner.write();
        inner
            .metadata()
            .and_then(|fi| {
                if fi.len() == end {
                    Ok(())
                } else {
                    assert!(!self.opt.read_only);
                    self.size.store(end as u32, Ordering::SeqCst);
                    inner.truncate(end)
                }
            })
            .map_err(From::from)
    }

    fn write_entry(&self, buf: &mut BytesMut, ent: &Entry) -> Result<()> {
        buf.clear();
        let mut inner = self.inner.write();
        let offset = self.write_at.load(Ordering::Acquire);
        self.encode_entry(buf, ent, offset).and_then(|encoded_len| {
            inner
                .write_all(buf.as_ref(), offset as usize)
                .map(|_| {
                    let new_write_at = encoded_len as u32 + offset;
                    self.write_at.store(new_write_at, Ordering::Release);
                    let start = new_write_at as usize;
                    inner.zero_range(start, start + MAX_HEADER_SIZE);
                })
                .map_err(From::from)
        })
    }

    /// Encode entry to the buf layout of entry
    ///
    /// ```text
    /// +--------+-----+-------+-------+
    /// | header | key | value | crc32 |
    /// +--------+-----+-------+-------+
    /// ```
    fn encode_entry(&self, buf: &mut BytesMut, ent: &Entry, offset: u32) -> Result<usize> {
        let header = ent.get_header();

        let mut hash = vpb::checksum::crc32fast::Hasher::default();
        // encode header.
        let mut header_enc = [0; MAX_HEADER_SIZE];
        let sz = header.encode(&mut header_enc);

        hash.update(&header_enc[..sz]);
        buf.put_slice(&header_enc[..sz]);

        // we'll encrypt only key and value.
        let key = ent.get_key();
        let k_len = key.len();
        let value = ent.get_value().parse_value();
        let v_len = value.len();
        match &self.data_key {
            Some(data_key) => {
                let mut ebuf = BytesMut::with_capacity(key.len() + value.len());
                ebuf.put_slice(key);
                ebuf.put_slice(value);
                ebuf.encrypt_to_vec(&data_key.data, &Self::generate_iv(&self.base_iv, offset), self.registry.encryption_algorithm())
                .map(|encrypted| {
                    hash.update(&encrypted);
                    buf.put_slice(&encrypted);
                    // write crc32 hash.
                    buf.put_u32(hash.finalize());
                    // return encoded length.
                    sz + k_len + v_len + core::mem::size_of::<u32>()
                })
                .map_err(|e| {
                    #[cfg(feature = "tracing")]
                    {
                        tracing::error!(target: "wal", err = %e, "failed to encrypt entry for vlog");
                    }
                    e.into()
                })
            }
            None => {
                // Encryption is disabled so writing directly to the buffer.
                hash.update(key);
                buf.put_slice(key);
                hash.update(value);
                buf.put_slice(value);
                // write crc32 hash.
                buf.put_u32(hash.finalize());
                // return encoded length.
                Ok(sz + k_len + v_len + core::mem::size_of::<u32>())
            }
        }
    }

    fn decode_entry(&self, src: &[u8], offset: u32) -> Result<Entry> {
        let (h_len, h) = Header::decode(src);
        let kv = &src[h_len..];

        let kv: Bytes = Self::decrypt_kv(
            kv,
            self.data_key.as_ref().map(|dk| dk.data.as_ref()),
            &self.base_iv,
            offset,
            self.registry.encryption_algorithm(),
        )?
        .into();

        let key_len = h.get_key_len() as usize;
        let val_len = h.get_value_len() as usize;
        let key = Key::from(kv.slice(..key_len));
        let val = Value::new()
            .set_data(kv.slice(key_len..key_len + val_len))
            .set_meta(h.get_meta())
            .set_expires_at(h.get_expires_at());
        let mut ent = Entry::new_from_kv(key, val);
        ent.set_offset(offset);
        Ok(ent)
    }

    /// Iterates over log file. It doesn't not allocate new memory for every kv pair.
    /// Therefore, the kv pair is only valid for the duration of fn call.
    #[inline]
    fn iter<F: FnMut(EntryRef, ValuePointer) -> Result<()>>(
        &self,
        mut offset: u32,
        log_entry: F,
    ) -> Result<u32> {
        if offset == 0 {
            // If offset is set to zero, let's advance past the encryption key header.
            offset = VALUE_LOG_HEADER_SIZE as u32;
        }

        let inner = self.inner.read();
        let iter = WALIterator {
            reader: &inner.as_slice()[(offset as usize)..],
            safe_reader: SafeRead {
                kv: Box::into_raw(Box::new(vec![])),
                record_offset: Cell::new(offset),
            },
            base_iv: self.base_iv.as_slice(),
            secret: self.data_key.as_ref().map(|dk| dk.data.as_ref()),
            path: &self.path,
            fid: self.fid,
            encryption_algorithm: self.registry.encryption_algorithm(),
            start_offset: offset,
        };

        iter.valid_end_offset(log_entry)
    }
}

impl WAL {
    pub(crate) fn wlock(&self) -> RwLockWriteGuard<MmapFileMut> {
        self.inner.write()
    }

    pub(crate) fn read_from_buffer(
        &self,
        buf: &mut BytesMut,
        start_offset: u32,
        end_offset: u32,
    ) -> Result<()> {
        let mut inner = self.inner.write();
        if (end_offset as usize) >= inner.len() {
            // Increase the file size if we cannot accommodate this entry.
            inner.truncate(end_offset as u64)?;
        }
        let start = end_offset - start_offset;
        inner.as_mut_slice()[start as usize..end_offset as usize].copy_from_slice(buf);

        self.size.store(end_offset, Ordering::SeqCst);
        Ok(())
    }

    pub(crate) fn read(&self, p: ValuePointer) -> Result<ReadGuard> {
        let mut num_bytes_read = 0;
        let offset = p.offset as u64;
        // Do not convert sz to uint32, because the self.mmap.len() can be of size
        // 4GB, which overflows the uint32 during conversion to make the size 0,
        // causing the read to fail with ErrEOF.
        let inner = self.inner.read();
        let sz = inner.len() as u64;
        let val_sz = p.len as u64;
        let self_sz = self.size.load(Ordering::SeqCst) as u64;

        if offset >= sz || offset + val_sz > sz || offset + val_sz > self_sz {
            // TODO: public fmmap::error::Error::new function
            #[cfg(feature = "metrics")]
            {
                use crate::metrics::{NUM_BYTES_READ, NUM_READS};
                if self.opt.enable_metrics {
                    NUM_READS.fetch_add(1, Ordering::SeqCst);
                    NUM_BYTES_READ.fetch_add(num_bytes_read, Ordering::SeqCst);
                }
            }
            Err(Error::EOF)
        } else {
            let offset = offset as usize;
            num_bytes_read = val_sz;
            #[cfg(feature = "metrics")]
            {
                use crate::metrics::{NUM_BYTES_READ, NUM_READS};
                if self.opt.enable_metrics {
                    NUM_READS.fetch_add(1, Ordering::SeqCst);
                    NUM_BYTES_READ.fetch_add(num_bytes_read, Ordering::SeqCst);
                }
            }
            Ok(ReadGuard {
                inner,
                start: offset,
                end: offset + val_sz as usize,
                val: EMPTY_BYTES,
            })
        }
    }

    fn open_in(
        path: PathBuf,
        fid: u32,
        mmap: MmapFileMut,
        registry: Registry,
        size: usize,
        opts: LogFileOptions,
    ) -> Result<Self> {
        if size < VALUE_LOG_HEADER_SIZE {
            // Every vlog file should have at least VALUE_LOG_HEADER_SIZE. If it is less than vlogHeaderSize
            // then it must have been corrupted. But no need to handle here. log replayer will truncate
            // and bootstrap the logfile. So ignoring here.
            return Err(Error::CorruptedLogFile);
        }

        // Copy over the encryption registry data.
        let mut buf = [0; VALUE_LOG_HEADER_SIZE];
        buf.copy_from_slice(mmap.slice(0, VALUE_LOG_HEADER_SIZE));
        let key_id = u64::from_be_bytes((&buf[..8]).try_into().unwrap());

        // retrieve datakey.
        registry.data_key(&key_id)
            .map(|dk| Self {
                inner: RwLock::new(mmap),
                path,
                fid,
                write_at: AtomicU32::new(VALUE_LOG_HEADER_SIZE as u32),
                size: AtomicU32::new(size as u32),
                data_key: dk,
                base_iv: buf[8..].try_into().unwrap(),
                registry,
                opt: opts,
            })
            .map_err(|e| {
                #[cfg(feature = "tracing")]
                {
                    tracing::error!(target: "wal", err = %e, "while opening value log file {}", fid);
                }
                e
            })
    }

    /// Initialize the log file with key id and baseIV.
    /// The below figure shows the layout of log file.
    ///
    /// ```text
    /// +----------------+------------------+------------------+
    /// | keyID(8 bytes) | baseIV(12 bytes) |     entry...     |
    /// +----------------+------------------+------------------+
    /// ```
    fn bootstrap(
        path: PathBuf,
        fid: u32,
        registry: Registry,
        mut mmap: MmapFileMut,
        opt: LogFileOptions,
    ) -> Result<Self> {
        // generate data key for the log file.
        let (key_id, data_key) = if registry.encryption_algorithm().is_none() {
            (0, None)
        } else {
            let data_key = registry.latest_data_key()?;
            (data_key.key_id, Some(data_key))
        };

        // We'll always preserve vlogHeaderSize for key id and baseIV.
        let mut buf = [0; VALUE_LOG_HEADER_SIZE as usize];
        // write key id to the buf.
        // key id will be zero if the logfile is in plain text.
        buf[..8].copy_from_slice(key_id.to_be_bytes().as_ref());
        // generate base IV. It'll be used with offset of the vptr to encrypt the entry.
        let mut rng = thread_rng();
        rng.fill_bytes(&mut buf[8..]);

        // Initialize base IV.
        let base_iv: [u8; WAL_IV_SIZE] = buf[8..].try_into().unwrap();

        // Copy over to the logFile.
        mmap.slice_mut(0, buf.len()).copy_from_slice(&buf);
        mmap.zero_range(
            VALUE_LOG_HEADER_SIZE,
            VALUE_LOG_HEADER_SIZE + MAX_HEADER_SIZE,
        );

        Ok(Self {
            inner: RwLock::new(mmap),
            path,
            fid,
            size: AtomicU32::new(VALUE_LOG_HEADER_SIZE as u32),
            write_at: AtomicU32::new(VALUE_LOG_HEADER_SIZE as u32),
            data_key,
            base_iv,
            registry,
            opt,
        })
    }

    pub(crate) fn done_writing(&self, offset: u32) -> Result<()> {
        if self.opt.sync_writes {
            self.inner.read().flush().map_err(|e| {
                #[cfg(feature = "tracing")]
                {
                    tracing::error!(target: "wal", err=%e, "unable to sync value log");
                }
                e
            })?;
        }

        // Before we were acquiring a lock here on lf.lock, because we were invalidating the file
        // descriptor due to reopening it as read-only. Now, we don't invalidate the fd, but unmap it,
        // truncate it and remap it. That creates a window where we have segfaults because the mmap is
        // no longer valid, while someone might be reading it. Therefore, we need a lock here again.
        self.truncate(offset as u64).map_err(|e| {
            #[cfg(feature = "tracing")]
            {
                tracing::error!(target: "wal", err=%e, "unable to truncate file");
            }
            e
        })
    }

    #[inline]
    pub(crate) fn secret(&self) -> Option<&[u8]> {
        self.data_key.as_ref().map(|dk| dk.data.as_ref())
    }

    #[inline]
    pub(crate) fn base_iv(&self) -> &[u8] {
        &self.base_iv
    }

    #[inline]
    pub(crate) fn encryption_algorithm(&self) -> EncryptionAlgorithm {
        self.registry.encryption_algorithm()
    }

    #[inline]
    pub(crate) fn decrypt_kv(
        src: &[u8],
        secret: Option<&[u8]>,
        base_iv: &[u8],
        offset: u32,
        algo: EncryptionAlgorithm,
    ) -> Result<Vec<u8>> {
        match secret {
            Some(secret) => src
                .encrypt_to_vec(secret, &Self::generate_iv(base_iv, offset), algo)
                .map_err(From::from),
            None => Ok(src.to_vec()),
        }
    }

    /// Returns datakey's ID.
    #[inline]
    fn key_id(&self) -> u64 {
        // If there is no datakey, then we'll return 0. Which means no encryption.
        self.data_key.as_ref().map(|k| k.key_id).unwrap_or(0)
    }

    /// Generates IV by appending given offset with the base IV.
    #[inline]
    pub(crate) fn generate_iv(base_iv: &[u8], offset: u32) -> [u8; BLOCK_SIZE] {
        let mut iv = [0u8; BLOCK_SIZE];
        // base_iv is of 12 bytes.
        iv[..WAL_IV_SIZE].copy_from_slice(base_iv);
        // remaining 4 bytes is obtained from offset.
        iv[WAL_IV_SIZE..].copy_from_slice(&offset.to_be_bytes());
        iv
    }
}

pub struct WALIterator<'a> {
    reader: &'a [u8],
    safe_reader: SafeRead,
    base_iv: &'a [u8],
    secret: Option<&'a [u8]>,
    encryption_algorithm: EncryptionAlgorithm,
    start_offset: u32,
    path: &'a Path,
    fid: u32,
}

impl<'a> WALIterator<'a> {
    #[inline]
    pub(crate) fn next(&self, cursor: usize) -> Result<Option<(usize, EntryRef)>> {
        match self.safe_reader.read_entry(
            &self.reader[cursor..],
            self.base_iv,
            self.secret,
            self.encryption_algorithm,
        ) {
            Ok(ent) => {
                if ent.1.get_key().is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(ent))
                }
            }
            Err(err) => match err {
                Error::Truncate => Ok(None),
                Error::EOF => Ok(None),
                err => Err(err),
            },
        }
    }

    /// Iterates over WAL file. It doesn't not allocate new memory for every kv pair.
    /// Therefore, the kv pair is only valid for the duration of fn call.
    #[inline]
    pub(crate) fn valid_end_offset<F: FnMut(EntryRef, ValuePointer) -> Result<()>>(
        self,
        mut log_entry: F,
    ) -> Result<u32> {
        // For now, read directly from file, because it allows
        let mut valid_end_offset = self.start_offset;
        let mut last_commit = 0;
        let mut vptrs = Vec::new();
        let mut entries = Vec::new();
        let fid = self.fid;
        let mut readed = 0;

        while let Some((bytes_read, ent)) = self.next(readed)? {
            readed += bytes_read;

            let vp = ValuePointer {
                fid,
                len: ent.get_offset(),
                offset: ent.get_offset(),
            };

            let ent_meta = ent.get_meta();
            let meta_txn = ent_meta & OP::BIT_TXN.bits();
            let meta_fin_txn = ent_meta & OP::BIT_FIN_TXN.bits();

            if meta_txn > 0 {
                let txn_ts = ent.parse_timestamp();
                if last_commit == 0 {
                    last_commit = txn_ts;
                }
                if last_commit != txn_ts {
                    break;
                }

                entries.push(ent);
                vptrs.push(vp);
            } else if meta_fin_txn > 0 {
                let vr = ent.val.parse_value();
                if vr.len() < 8 {
                    break;
                }

                let txn_ts = u64::from_be_bytes((&vr[..8]).try_into().unwrap());
                if last_commit != txn_ts {
                    break;
                }

                // Got the end of txn. Now we can store them.
                last_commit = 0;
                valid_end_offset = ent.get_offset();

                for (i, e) in entries.iter().enumerate() {
                    let vp = vptrs[i];
                    if let Err(err) = log_entry(*e, vp) {
                        match err {
                            Error::Stop => break,
                            err => {
                                #[cfg(feature = "tracing")]
                                {
                                    tracing::error!(target: "wal", err = %err, "Iteration function. Path={}.", self.path.display());
                                }
                                return Err(err);
                            }
                        }
                    }
                }
                entries.clear();
                vptrs.clear();
            } else {
                if last_commit != 0 {
                    // This is most likely an entry which was moved as part of GC.
                    // We shouldn't get this entry in the middle of a transaction.
                    break;
                }

                valid_end_offset = ent.get_offset();

                if let Err(err) = log_entry(ent, vp) {
                    match err {
                        Error::Stop => break,
                        err => {
                            #[cfg(feature = "tracing")]
                            {
                                tracing::error!(target: "wal", err = %err, "Iteration function. Path={}.", self.path.display());
                            }
                            return Err(err);
                        }
                    }
                }
            }
        }
        Ok(valid_end_offset)
    }
}

#[cfg(test)]
mod test {
    use crate::test::get_registry_test_options;

    use super::*;
    use scopeguard::defer;
    use std::fs;
    use std::sync::atomic::AtomicUsize;
    use vpb::prost::Message;
    use vpb::Encryption;

    const MEM_TABLE_SIZE: u64 = 64 << 20; // 64 MB

    fn get_wal_pathbuf(mut dir: PathBuf, name: &str) -> PathBuf {
        dir.push(name);
        dir.set_extension("wal");
        dir
    }

    fn get_test_wal_options(save_on_drop: bool) -> LogFileOptions {
        LogFileOptions {
            sync_writes: true,
            read_only: false,
            enable_metrics: false,
            mem_table_size: MEM_TABLE_SIZE,
            save_on_drop,
        }
    }

    #[test]
    fn test_create_wal() {
        let mut ek: [u8; 32] = Default::default();
        let mut rng = rand::thread_rng();
        rng.fill_bytes(&mut ek);

        let mut dir = std::env::temp_dir();
        dir.push("vela-test-create-wal");
        fs::create_dir(dir.clone()).unwrap();

        let dd = dir.clone();
        defer!(fs::remove_dir_all(dd).unwrap(););

        let opts = get_registry_test_options(&dir.clone(), Encryption::aes(ek.to_vec()));
        let kr = Registry::open(opts).unwrap();

        WAL::open(
            get_wal_pathbuf(dir, "test"),
            0,
            0,
            kr,
            get_test_wal_options(false),
        )
        .unwrap();
    }

    #[test]
    fn test_header_encode_decode() {
        let h = Header::new().set_key_len(3).set_value_len(3);

        let (hel, buf) = h.encode_to_vec();
        let (hdl, hd) = Header::decode(&buf);
        assert_eq!(hel, hdl);
        assert_eq!(h, hd);
    }

    fn wal_valid_end_offset(encrypt: bool, suffix: usize) {
        let ctr: AtomicUsize = AtomicUsize::new(0);
        const N: usize = 10;

        let mut dir = std::env::temp_dir();
        dir.push(format!("vela-test-wal-valid_end_offset_{}", suffix));
        fs::create_dir(dir.clone()).unwrap();
        defer!(fs::remove_dir_all(&dir).unwrap(););

        let ek = if encrypt {
            let mut ek: [u8; 32] = Default::default();
            let mut rng = rand::thread_rng();
            rng.fill_bytes(&mut ek);
            Encryption::aes(ek.to_vec())
        } else {
            Encryption::new()
        };
        let opts = get_registry_test_options(&dir, ek);
        let dir = opts.dir().clone();
        let kr = Registry::open(opts).unwrap();

        let w = WAL::open(
            get_wal_pathbuf(dir.clone(), suffix.to_string().as_str()),
            0,
            0,
            kr.clone(),
            get_test_wal_options(true),
        )
        .unwrap();

        let mut buf = BytesMut::new();

        for i in 0..N {
            let ent = Entry::new_from_kv(
                Key::from(i.to_string().encode_to_vec()),
                Value::from(i.to_string().encode_to_vec()),
            );
            w.write_entry(&mut buf, &ent).unwrap();
        }
        drop(w);

        let w = WAL::open(
            get_wal_pathbuf(dir, suffix.to_string().as_str()),
            0,
            0,
            kr,
            get_test_wal_options(false),
        )
        .unwrap();

        w.iter(0, |e, _vp| {
            let i = ctr.fetch_add(1, Ordering::Relaxed);
            assert_eq!(e.key.as_slice(), i.to_string().encode_to_vec().as_slice());
            assert_eq!(
                e.val.parse_value(),
                i.to_string().encode_to_vec().as_slice()
            );
            Ok(())
        })
        .unwrap();

        assert_eq!(ctr.load(Ordering::Relaxed), N);
    }

    #[test]
    fn test_wal_valid_end_offset() {
        wal_valid_end_offset(true, 0);
        wal_valid_end_offset(false, 1);
    }

    #[test]
    fn test_wal_encode_decode() {
        let mut ek: [u8; 32] = Default::default();
        let mut rng = rand::thread_rng();
        rng.fill_bytes(&mut ek);

        let mut dir = std::env::temp_dir();
        dir.push("vela-test-wal-encode-decode");
        fs::create_dir(dir.clone()).unwrap();
        defer!(fs::remove_dir_all(&dir).unwrap(););

        let opts = get_registry_test_options(&dir, Encryption::aes(ek.to_vec()));
        let dir = opts.dir().clone();
        let kr = Registry::open(opts).unwrap();

        let w = WAL::open(
            get_wal_pathbuf(dir, "test"),
            0,
            0,
            kr,
            get_test_wal_options(false),
        )
        .unwrap();
        let mut buf = BytesMut::new();
        let ent = Entry::new_from_kv(
            Key::from(2.to_string().encode_to_vec()),
            Value::from(2.to_string().encode_to_vec()),
        );
        w.encode_entry(&mut buf, &ent, 0).unwrap();
        let ent1 = w.decode_entry(buf.as_ref(), 0).unwrap();
        assert_eq!(ent.get_key().as_slice(), ent1.get_key().as_slice());
        assert_eq!(ent.get_value(), ent1.get_value());
    }
}
