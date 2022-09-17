use core::sync::atomic::{AtomicU32, Ordering};
use std::path::PathBuf;

use fmmap::{MetaDataExt, MmapFileExt, MmapFileMut, MmapFileMutExt, Options};
use parking_lot::RwLock;
use rand::{thread_rng, RngCore};
use vpb::{
    encrypt::{Encryptor, BLOCK_SIZE},
    kvstructs::{
        bytes::{BufMut, Bytes, BytesMut},
        Entry, Header, Key, Value, ValueExt, MAX_HEADER_SIZE,
    },
    DataKey,
};

use crate::{error::*, registry::Registry, ValuePointer, VALUE_LOG_HEADER_SIZE};

const WAL_IV_SIZE: usize = 12;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WALOptions {
    sync_writes: bool,
    read_only: bool,
    enable_metrics: bool,
    mem_table_size: u64,
}

pub struct WAL {
    inner: RwLock<WALInner>,
    path: PathBuf,
    fid: u32,
    size: AtomicU32,
    data_key: Option<DataKey>,
    base_iv: [u8; WAL_IV_SIZE],
    registry: Registry,
    opt: WALOptions,
}

struct WALInner {
    mmap: MmapFileMut,
    write_at: u32,
}

impl WAL {
    pub fn open<P: AsRef<std::path::Path>>(
        path: P,
        #[cfg(unix)] flags: i32,
        #[cfg(windows)] flags: u32,
        fid: u32,
        registry: Registry,
        opts: WALOptions,
    ) -> Result<Option<Self>> {
        if !path.as_ref().exists() {
            Options::new()
                .max_size(opts.mem_table_size * 2)
                .custom_flags(flags)
                .create_mmap_file_mut(&path)
                .map_err(From::from)
                .and_then(|mut mmap| {
                    mmap.set_remove_on_drop(true);
                    Self::bootstrap(path.as_ref().to_path_buf(), fid, registry, mmap, opts)
                        .map(Option::Some)
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
                .and_then(|mmap| {
                    let size = mmap.len();
                    Self::open_in(path.as_ref().to_path_buf(), fid, mmap, registry, size, opts)
                })
        }
    }

    fn open_in(
        path: PathBuf,
        fid: u32,
        mmap: MmapFileMut,
        registry: Registry,
        size: usize,
        opts: WALOptions,
    ) -> Result<Option<Self>> {
        if size < VALUE_LOG_HEADER_SIZE {
            // Every vlog file should have at least VALUE_LOG_HEADER_SIZE. If it is less than vlogHeaderSize
            // then it must have been corrupted. But no need to handle here. log replayer will truncate
            // and bootstrap the logfile. So ignoring here.
            return Ok(None);
        }

        // Copy over the encryption registry data.
        let mut buf = [0; VALUE_LOG_HEADER_SIZE];
        buf.copy_from_slice(mmap.slice(0, VALUE_LOG_HEADER_SIZE));
        let key_id = u64::from_be_bytes((&buf[..8]).try_into().unwrap());

        // retrieve datakey.
        registry.data_key(&key_id)
            .map(|dk| Some(Self {
                inner: RwLock::new(WALInner {
                    mmap,
                    write_at: VALUE_LOG_HEADER_SIZE as u32,
                }),
                path,
                fid,
                size: AtomicU32::new(size as u32),
                data_key: dk,
                base_iv: buf[8..].try_into().unwrap(),
                registry,
                opt: opts,
            }))
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
        opt: WALOptions,
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
            inner: RwLock::new(WALInner {
                mmap,
                write_at: VALUE_LOG_HEADER_SIZE as u32,
            }),
            path,
            fid,
            size: AtomicU32::new(VALUE_LOG_HEADER_SIZE as u32),
            data_key,
            base_iv,
            registry,
            opt,
        })
    }

    pub fn truncate(&self, end: u64) -> Result<()> {
        let mut inner = self.inner.write();
        inner
            .mmap
            .metadata()
            .and_then(|fi| {
                if fi.len() == end {
                    Ok(())
                } else {
                    assert!(!self.opt.read_only);
                    self.size.store(end as u32, Ordering::SeqCst);
                    inner.mmap.truncate(end)
                }
            })
            .map_err(From::from)
    }

    /// Iterates over log file. It doesn't not allocate new memory for every kv pair.
    /// Therefore, the kv pair is only valid for the duration of fn call.
    pub(crate) fn iter(&self, offset: u32) -> Result<u32> {
        todo!()
    }

    pub(crate) fn read(&self, p: ValuePointer) -> Result<Vec<u8>> {
        let mut num_bytes_read = 0;
        let offset = p.offset as u64;
        // Do not convert sz to uint32, because the self.mmap.len() can be of size
        // 4GB, which overflows the uint32 during conversion to make the size 0,
        // causing the read to fail with ErrEOF.
        let inner = self.inner.read();
        let sz = inner.mmap.len() as u64;
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
            let buf = inner.mmap.as_slice()[offset..offset + val_sz as usize].to_vec();
            num_bytes_read = val_sz;
            #[cfg(feature = "metrics")]
            {
                use crate::metrics::{NUM_BYTES_READ, NUM_READS};
                if self.opt.enable_metrics {
                    NUM_READS.fetch_add(1, Ordering::SeqCst);
                    NUM_BYTES_READ.fetch_add(num_bytes_read, Ordering::SeqCst);
                }
            }
            Ok(buf)
        }
    }

    pub(crate) fn write_entry(&self, buf: &mut BytesMut, ent: &Entry) -> Result<()> {
        buf.clear();
        let mut inner = self.inner.write();
        let offset = inner.write_at;
        self.encode_entry(buf, ent, offset).and_then(|encoded_len| {
            inner
                .mmap
                .write_all(buf.as_ref(), offset as usize)
                .map(|_| {
                    inner.write_at += encoded_len as u32;
                    inner
                        .mmap
                        .zero_range(offset as usize, offset as usize + MAX_HEADER_SIZE);
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
    pub(crate) fn encode_entry(
        &self,
        buf: &mut BytesMut,
        ent: &Entry,
        offset: u32,
    ) -> Result<usize> {
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
                ebuf.encrypt_to_vec(&data_key.data, &self.generate_iv(offset), self.registry.encryption_algorithm())
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

    pub(crate) fn decode_entry(&self, src: &[u8], offset: u32) -> Result<Entry> {
        let (h_len, h) = Header::decode(src);
        let kv = &src[h_len..];

        let kv: Bytes = self.decrypt_kv(kv, offset)?.into();

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

    fn done_writing(&self, offset: u32) -> Result<()> {
        if self.opt.sync_writes {
            self.inner.read().mmap.flush().map_err(|e| {
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
    fn decrypt_kv(&self, src: &[u8], offset: u32) -> Result<Vec<u8>> {
        src.encrypt_to_vec(
            self.data_key
                .as_ref()
                .map(|dk| dk.data.as_ref())
                .unwrap_or(&[]),
            &self.generate_iv(offset),
            self.registry.encryption_algorithm(),
        )
        .map_err(From::from)
    }

    /// Returns datakey's ID.
    #[inline]
    fn key_id(&self) -> u64 {
        // If there is no datakey, then we'll return 0. Which means no encryption.
        self.data_key.as_ref().map(|k| k.key_id).unwrap_or(0)
    }

    /// Generates IV by appending given offset with the base IV.
    #[inline]
    fn generate_iv(&self, offset: u32) -> [u8; BLOCK_SIZE] {
        let mut iv = [0u8; BLOCK_SIZE];
        // base_iv is of 12 bytes.
        iv[..WAL_IV_SIZE].copy_from_slice(&self.base_iv);
        // remaining 4 bytes is obtained from offset.
        iv[WAL_IV_SIZE..].copy_from_slice(&offset.to_be_bytes());
        iv
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

    const MEM_TABLE_SIZE: u64 = 64 << 20; // 64 MB

    fn get_wal_pathbuf(mut dir: PathBuf, name: &str) -> PathBuf {
        dir.push(name);
        dir.set_extension("wal");
        dir
    }

    fn get_test_wal_options() -> WALOptions {
        WALOptions {
            sync_writes: true,
            read_only: false,
            enable_metrics: false,
            mem_table_size: MEM_TABLE_SIZE,
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

        let opts = get_registry_test_options(&dir.clone(), ek.to_vec());
        let kr = Registry::open(opts).unwrap();

        WAL::open(
            get_wal_pathbuf(dir, "test"),
            0,
            0,
            kr,
            get_test_wal_options(),
        )
        .unwrap()
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
        const N: usize = 100;

        let mut dir = std::env::temp_dir();
        dir.push(format!("vela-test-wal-valid_end_offset_{}", suffix));
        fs::create_dir(dir.clone()).unwrap();
        defer!(fs::remove_dir_all(&dir).unwrap(););

        let ek = if encrypt {
            let mut ek: [u8; 32] = Default::default();
            let mut rng = rand::thread_rng();
            rng.fill_bytes(&mut ek);
            ek.to_vec()
        } else {
            vec![]
        };
        let opts = get_registry_test_options(&dir, ek);
        let dir = opts.dir().clone();
        let kr = Registry::open(opts).unwrap();

        let mut w = WAL::open(
            get_wal_pathbuf(dir.clone(), suffix.to_string().as_str()),
            0,
            0,
            kr.clone(),
            get_test_wal_options(),
        )
        .unwrap()
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

        let mut w = WAL::open(
            get_wal_pathbuf(dir, suffix.to_string().as_str()),
            0,
            0,
            kr,
            get_test_wal_options(),
        )
        .unwrap()
        .unwrap();

        // let mut iter = w.iter(0);
        // let _ = iter
        //     .valid_end_offset(|e, _vp| {
        //         let i = ctr.fetch_add(1, Ordering::Relaxed);
        //         assert_eq!(e.key.key().as_slice(), i.to_string().encode_to_vec().as_slice());
        //         assert_eq!(e.val.value(), i.to_string().encode_to_vec().as_slice());
        //         Ok(())
        //     })
        //     .unwrap();
        // assert_eq!(ctr.load(Ordering::Relaxed), N);
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

        let opts = get_registry_test_options(&dir, ek.to_vec());
        let dir = opts.dir().clone();
        let kr = Registry::open(opts).unwrap();

        let w = WAL::open(
            get_wal_pathbuf(dir, "test"),
            0,
            0,
            kr,
            get_test_wal_options(),
        )
        .unwrap()
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
