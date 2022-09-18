use crate::open_trunc_file;

use super::error::*;
use core::{
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};
use parking_lot::{RwLock, RwLockWriteGuard};
use std::{
    collections::{hash_map::Entry, HashMap},
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
use vela_utils::ref_counter::RefCounter;
use vpb::{
    encrypt::{is_valid_key_length, random_iv, Encryptor},
    kvstructs::bytes::{Bytes, BytesMut},
    prost::bytes::BufMut,
    DataKey, Encryption, EncryptionAlgorithm, Marshaller, IV,
};

/// The file name for the key registry file.
pub const REGISTRY_FILENAME: &str = "REGISTRY";

/// The file extension for the key registry file.
pub const REGISTRY_FILE_EXTENSION: &str = "kr";

/// The file name for the rewrite key registry file.
pub const REWRITE_REGISTRY_FILENAME: &str = "REWRITE-REGISTRY";

/// The file extension for the rewrite key registry file.
pub const REWRITE_REGISTRY_FILE_EXTENSION: &str = "krr";

const SANITY_TEXT: &[u8] = b"Hello VelaDB";

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RegistryOptions {
    dir: PathBuf,
    in_memory: bool,
    read_only: bool,
    encryption_key_rotation_duration: Duration,
    encryption: Encryption,
}

impl Default for RegistryOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl RegistryOptions {
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

/// Structure of Key Registry.
/// This structure is lock free.
///
/// ```text
/// +-------------------+---------------------+--------------------+--------------+------------------+
/// |     IV            | Sanity Text         | DataKey1           | DataKey2     | ...              |
/// +-------------------+---------------------+--------------------+--------------+------------------+
/// ```
#[derive(Clone)]
pub enum Registry {
    Memory(RefCounter<MemoryRegistry>),
    Persistent(RefCounter<PersistentRegistry>),
}

pub struct MemoryRegistry {
    inner: RwLock<MemoryRegistryInner>,
    opts: RefCounter<RegistryOptions>,
    // record number of the keys in the registry, avoid lock the whole registry.
    num_of_keys: AtomicUsize,
}

struct MemoryRegistryInner {
    data_keys: HashMap<u64, DataKey>,
    last_created: u64, // last_created is the timestamp(seconds) of the last data key generated.
    next_key_id: u64,
}

impl MemoryRegistry {
    #[inline]
    fn new(opts: RefCounter<RegistryOptions>) -> Self {
        Self {
            inner: RwLock::new(MemoryRegistryInner {
                data_keys: HashMap::new(),
                last_created: 0,
                next_key_id: 0,
            }),
            opts,
            num_of_keys: AtomicUsize::new(0),
        }
    }

    /// Returns `DataKey` of the given key id.
    #[inline]
    pub fn data_key(&self, id: &u64) -> Result<Option<DataKey>> {
        let inner = self.inner.read();
        if id.eq(&0) {
            // nil represent plain text.
            return Ok(None);
        }

        match inner.data_keys.get(id) {
            Some(data_key) => Ok(Some(data_key.clone())),
            None => Err(Error::InvalidDataKeyID(*id)),
        }
    }

    #[inline]
    pub fn insert(&self, mut dk: DataKey) -> Result<u64> {
        let mut key_id = dk.key_id;
        let mut inner = self.inner.write();
        let next_key_id = inner.next_key_id + 1;
        match inner.data_keys.entry(key_id) {
            Entry::Occupied(mut entry) => {
                entry.insert(dk);
            }
            Entry::Vacant(entry) => {
                dk.key_id = next_key_id;
                key_id = dk.key_id;
                entry.insert(dk);
            }
        }
        self.num_of_keys.fetch_add(1, Ordering::SeqCst);
        Ok(key_id)
    }

    /// Give you the latest generated datakey based on the rotation
    /// period. If the last generated datakey lifetime exceeds the rotation period.
    /// It'll create new datakey.
    pub fn latest_data_key(&self) -> Result<DataKey> {
        let inner = self.inner.read();
        if valid_key(
            inner.last_created,
            self.opts.encryption_key_rotation_duration,
        ) {
            // If less than EncryptionKeyRotationDuration, returns the last generated key.
            return Ok(inner.data_keys.get(&inner.next_key_id).unwrap().clone());
        }
        drop(inner);

        let mut inner = self.inner.write();
        // Key might have generated by another go routine. So,
        // checking once again.
        if valid_key(
            inner.last_created,
            self.opts.encryption_key_rotation_duration,
        ) {
            return Ok(inner.data_keys.get(&inner.next_key_id).unwrap().clone());
        }

        inner.next_key_id += 1;
        let next_key_id = inner.next_key_id;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        if self.opts.encryption().is_none() {
            let dk = DataKey {
                key_id: inner.next_key_id,
                data: Bytes::new(),
                iv: IV::new(),
                created_at: now,
            };
            inner.last_created = now;
            inner.data_keys.insert(next_key_id, dk.clone());
            self.num_of_keys.fetch_add(1, Ordering::SeqCst);
            Ok(dk)
        } else {
            use rand::RngCore;

            let mut k = vec![0; self.opts.secret().len()];
            rand::thread_rng().fill_bytes(&mut k);

            let dk = DataKey {
                key_id: inner.next_key_id,
                data: Bytes::from(k),
                iv: IV::random(),
                created_at: now,
            };
            inner.last_created = now;
            inner.data_keys.insert(next_key_id, dk.clone());
            self.num_of_keys.fetch_add(1, Ordering::SeqCst);
            Ok(dk)
        }
    }
}

pub struct PersistentRegistry {
    inner: RwLock<PersistentRegistryInner>,
    opts: RefCounter<RegistryOptions>,
    // record number of the keys in the registry, avoid lock the whole registry.
    num_of_keys: AtomicUsize,
}

struct PersistentRegistryInner {
    data_keys: HashMap<u64, DataKey>,
    last_created: u64, // last_created is the timestamp(seconds) of the last data key generated.
    next_key_id: u64,
    file: File,
}

impl PersistentRegistry {
    /// Returns `DataKey` of the given key id.
    #[inline]
    pub fn data_key(&self, id: &u64) -> Result<Option<DataKey>> {
        let inner = self.inner.read();
        if id.eq(&0) {
            // nil represent plain text.
            return Ok(None);
        }

        match inner.data_keys.get(id) {
            Some(data_key) => Ok(Some(data_key.clone())),
            None => Err(Error::InvalidDataKeyID(*id)),
        }
    }

    #[inline]
    pub fn insert(&self, mut dk: DataKey) -> Result<u64> {
        let mut inner = self.inner.write();
        inner.next_key_id += 1;
        let next_key_id = inner.next_key_id;
        match inner.data_keys.entry(dk.key_id) {
            Entry::Occupied(mut entry) => {
                entry.insert(dk.clone());
            }
            Entry::Vacant(entry) => {
                dk.key_id = next_key_id;
                entry.insert(dk.clone());
            }
        }

        Self::store_data_key(inner, dk, &self.opts).map(|id| {
            self.num_of_keys.fetch_add(1, Ordering::SeqCst);
            id
        })
    }

    /// Give you the latest generated datakey based on the rotation
    /// period. If the last generated datakey lifetime exceeds the rotation period.
    /// It'll create new datakey.
    pub fn latest_data_key(&self) -> Result<DataKey> {
        let inner = self.inner.read();
        if valid_key(
            inner.last_created,
            self.opts.encryption_key_rotation_duration,
        ) {
            // If less than EncryptionKeyRotationDuration, returns the last generated key.
            return Ok(inner.data_keys.get(&inner.next_key_id).unwrap().clone());
        }
        drop(inner);

        let mut inner = self.inner.write();
        // Key might have generated by another go routine. So,
        // checking once again.
        if valid_key(
            inner.last_created,
            self.opts.encryption_key_rotation_duration,
        ) {
            return Ok(inner.data_keys.get(&inner.next_key_id).unwrap().clone());
        }

        inner.next_key_id += 1;
        let next_key_id = inner.next_key_id;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if self.opts.encryption().is_none() {
            let dk = DataKey {
                key_id: next_key_id,
                data: Bytes::new(),
                iv: IV::new(),
                created_at: now,
            };
            inner.last_created = now;
            inner.data_keys.insert(next_key_id, dk.clone());
            // Store the datekey.
            Self::store_data_key(inner, dk.clone(), &self.opts).map(|_| {
                self.num_of_keys.fetch_add(1, Ordering::SeqCst);
                dk
            })
        } else {
            use rand::RngCore;

            let mut k = vec![0; self.opts.secret().len()];
            rand::thread_rng().fill_bytes(&mut k);

            let dk = DataKey {
                key_id: next_key_id,
                data: Bytes::from(k),
                iv: IV::random(),
                created_at: now,
            };
            inner.last_created = now;
            inner.data_keys.insert(next_key_id, dk.clone());
            Self::store_data_key(inner, dk.clone(), &self.opts).map(|_| {
                self.num_of_keys.fetch_add(1, Ordering::SeqCst);
                dk
            })
        }
    }

    /// stores datakey in an encrypted format in the given buffer. If storage key preset.
    /// DO NOT use a pointer for key. storeDataKey may modify the kv.data field.
    #[inline]
    fn store_data_key(
        mut inner: RwLockWriteGuard<PersistentRegistryInner>,
        mut dk: DataKey,
        opts: &RegistryOptions,
    ) -> Result<u64> {
        dk.data
            .encrypt_to_vec(opts.secret(), dk.iv.as_ref(), opts.encryption.algorithm())
            .map_err(From::from)
            .and_then(|data| {
                dk.data = data.into();
                let data = dk.marshal();
                let mut buf = BytesMut::with_capacity(8 + data.len());
                buf.put_u32(data.len() as u32);
                buf.put_u32(vpb::checksum::crc32fast::hash(&data));
                buf.put_slice(&data);
                inner
                    .file
                    .write_all(&buf)
                    .map_err(From::from)
                    .map(|_| dk.key_id)
            })
    }
}

impl Registry {
    #[inline]
    pub fn encryption_algorithm(&self) -> EncryptionAlgorithm {
        match &self {
            Registry::Memory(r) => r.opts.encryption.algorithm(),
            Registry::Persistent(r) => r.opts.encryption.algorithm(),
        }
    }

    /// Opens key registry if it exists, otherwise it'll create key registry
    /// and returns key registry.
    pub fn open(opts: RefCounter<RegistryOptions>) -> Result<Self> {
        if opts.encryption.is_some() {
            let secret = opts.encryption.secret();
            // sanity check the encryption key length.
            if !is_valid_key_length(secret, opts.encryption.algorithm()) {
                #[cfg(feature = "tracing")]
                {
                    tracing::error!(target: "key_registry", "during open registry: invalid encryption key length {}", secret.len());
                }
                return Err(Error::InvalidEncryptionKeyLength(secret.len()));
            }
        }

        // If db is opened in InMemory mode, we don't need to write key registry to the disk.
        if opts.in_memory {
            return Ok(Registry::Memory(RefCounter::new(MemoryRegistry::new(opts))));
        }

        let open_opts = if opts.read_only {
            let mut opts = OpenOptions::new();
            opts.create(true).read(true).write(false);
            opts
        } else {
            let mut opts = OpenOptions::new();
            opts.create(true).read(true).write(true);
            opts
        };

        let path = opts
            .dir
            .join(REGISTRY_FILENAME)
            .with_extension(REGISTRY_FILE_EXTENSION);
        // registry file does not exist, we create a new one
        if !path.exists() {
            let mut file = open_opts.open(&path)?;

            if opts.encryption.is_none() {
                file.write_all(SANITY_TEXT)?;
            } else {
                let iv = random_iv();
                file.write_all(&iv)?;
                let e_sanity =
                    SANITY_TEXT.encrypt_to_vec(opts.secret(), &iv, opts.encryption.algorithm())?;
                file.write_all(&e_sanity)?;
            }

            file.flush().map_err(From::from).map(|_| {
                if opts.read_only {
                    Registry::Memory(RefCounter::new(MemoryRegistry::new(opts)))
                } else {
                    Registry::Persistent(RefCounter::new(PersistentRegistry {
                        inner: RwLock::new(PersistentRegistryInner {
                            data_keys: HashMap::new(),
                            file,
                            last_created: 0,
                            next_key_id: 0,
                        }),
                        opts,
                        num_of_keys: AtomicUsize::new(0),
                    }))
                }
            })
        } else {
            open_opts
                .open(&path)
                .map_err(From::from)
                .and_then(|file| Self::read_key_registry(file, opts))
        }
    }

    /// Rewrite the existing key registry file with new one.
    /// It is okay to give closed key registry. Since, it's using only the datakey.
    pub fn rewrite(&self, opts: RefCounter<RegistryOptions>) -> Result<()> {
        let mut buf = BytesMut::new();
        let iv = IV::random();

        // Encrypt sanity text if the encryption key is presents.
        let e_sanity = SANITY_TEXT.encrypt_to_vec(
            opts.secret(),
            iv.as_ref(),
            opts.encryption().algorithm(),
        )?;
        buf.put_slice(iv.as_ref());
        buf.put_slice(e_sanity.as_ref());

        // Write all the datakeys to the buf.
        match self {
            Registry::Memory(reg) => {
                let inner = reg.inner.read();
                // Writing the datakey to the given buffer.
                for (_, dk) in inner.data_keys.iter() {
                    store_data_key(&mut buf, dk.clone(), &opts).map_err(|e| {
                        #[cfg(feature = "tracing")]
                        {
                            tracing::error!(target: "key_registry", err = %e, "Error while storing datakey in Registry::rewrite");
                        }
                        e
                    })?;
                }
            }
            Registry::Persistent(reg) => {
                let reg = reg.inner.read();
                for (_, dk) in reg.data_keys.iter() {
                    store_data_key(&mut buf, dk.clone(), &opts).map_err(|e| {
                        #[cfg(feature = "tracing")]
                        {
                            tracing::error!(target: "key_registry", err = %e, "Error while storing datakey in Registry::rewrite");
                        }
                        e
                    })?;
                }
            }
        }

        let mut tmp_path = opts.dir.clone();
        tmp_path.push(REWRITE_REGISTRY_FILENAME);
        tmp_path.set_extension(REWRITE_REGISTRY_FILE_EXTENSION);
        // Open temporary file to write the data and do atomic rename.
        open_trunc_file(&tmp_path, true)
            .and_then(|mut f| f.write_all(buf.as_ref()).map(|_| f))
            .and_then(|f| f.sync_all())
            .and_then(|_| {
                let mut path = opts.dir.clone();
                path.push(REGISTRY_FILENAME);
                path.set_extension(REGISTRY_FILE_EXTENSION);
                std::fs::rename(&tmp_path, path)
            })
            .and_then(|_| File::open(&opts.dir))
            .and_then(|f| f.sync_all())
            .map_err(|e| {
                #[cfg(feature = "tracing")]
                {
                    tracing::error!(target: "key_registry", err = %e, "Error while rewriting the key registry");
                }
                e.into()
            })
    }

    /// Returns `DataKey` of the given key id.
    #[inline]
    pub fn data_key(&self, id: &u64) -> Result<Option<DataKey>> {
        match self {
            Registry::Memory(inner) => inner.data_key(id),
            Registry::Persistent(inner) => inner.data_key(id),
        }
    }

    /// Returns the number of data keys in the registry.
    #[inline]
    pub fn num_data_keys(&self) -> usize {
        match self {
            Registry::Memory(r) => r.num_of_keys.load(Ordering::Relaxed),
            Registry::Persistent(r) => r.num_of_keys.load(Ordering::Relaxed),
        }
    }

    /// Give you the latest generated datakey based on the rotation
    /// period. If the last generated datakey lifetime exceeds the rotation period.
    /// It'll create new datakey.
    #[inline]
    pub fn latest_data_key(&self) -> Result<DataKey> {
        match self {
            Registry::Memory(r) => r.latest_data_key(),
            Registry::Persistent(r) => r.latest_data_key(),
        }
    }

    #[inline]
    pub fn insert(&self, dk: DataKey) -> Result<u64> {
        match self {
            Registry::Memory(r) => r.insert(dk),
            Registry::Persistent(r) => r.insert(dk),
        }
    }

    /// Read the key registry file and build the key registry struct.
    fn read_key_registry(mut file: File, opts: RefCounter<RegistryOptions>) -> Result<Self> {
        let mut iter = RegistryIterator::new(&mut file, &opts.encryption)?;
        let mut map = HashMap::new();
        let mut next_key_id = 0;
        let mut last_created = 0;
        let mut dk = iter.next();

        let err = loop {
            match dk {
                Ok(datakey) => {
                    // Set the maximum key ID for next key ID generation.
                    next_key_id = next_key_id.max(datakey.key_id);
                    // Set the last generated key timestamp.
                    last_created = last_created.max(datakey.created_at);
                    map.insert(datakey.key_id, datakey);
                    dk = iter.next();
                }
                Err(e) => {
                    break e;
                }
            }
        };

        let num_of_keys = map.len();
        match err {
            // We read all the key. So, Ignoring this error.
            Error::IO(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                if opts.in_memory {
                    Ok(Registry::Memory(RefCounter::new(MemoryRegistry {
                        opts,
                        inner: RwLock::new(MemoryRegistryInner {
                            data_keys: map,
                            last_created,
                            next_key_id,
                        }),
                        num_of_keys: AtomicUsize::new(num_of_keys),
                    })))
                } else {
                    Ok(Registry::Persistent(RefCounter::new(PersistentRegistry {
                        inner: RwLock::new(PersistentRegistryInner {
                            data_keys: map,
                            file,
                            last_created,
                            next_key_id,
                        }),
                        opts,
                        num_of_keys: AtomicUsize::new(num_of_keys),
                    })))
                }
            }
            e => Err(e),
        }
    }

    pub fn valid_sanity(
        e_sanity_txt: &[u8],
        secret: &[u8],
        iv: &[u8],
        algo: EncryptionAlgorithm,
    ) -> Result<bool> {
        e_sanity_txt
            .encrypt_to_vec(secret, iv, algo)
            .map(|v| v.eq(SANITY_TEXT))
            .map_err(core::convert::From::from)
    }
}

const BUF_SIZE: usize = 8;

struct RegistryIterator<'a> {
    file: &'a mut File,
    encryption: &'a Encryption,
    crc_buf: [u8; BUF_SIZE],
}

impl<'a> RegistryIterator<'a> {
    #[inline]
    fn new(file: &'a mut File, encryption: &'a Encryption) -> Result<Self> {
        Self::read_meta(file, encryption).map(|file| Self {
            file,
            encryption,
            crc_buf: [0; BUF_SIZE],
        })
    }

    fn next(&mut self) -> Result<DataKey> {
        // Read crc buf and data length.
        self.file.read_exact(&mut self.crc_buf).map_err(|e| {
            // EOF means end of the iteration.
            if e.kind() == std::io::ErrorKind::UnexpectedEof {
                #[cfg(feature = "tracing")]
                {
                    tracing::error!(target: "key_registry", "while reading crc in RegistryIterator::next");
                }
                e
            } else {
                #[cfg(feature = "tracing")]
                {
                    tracing::error!(target: "key_registry", err=%e);
                }
                e
            }
        })?;
        // Read protobuf data.
        let l = u32::from_be_bytes(self.crc_buf[0..4].try_into().unwrap()) as usize;
        let mut data = vec![0; l];
        self.file.read_exact(&mut data).map_err(|e| {
            // EOF means end of the iteration.
            if e.kind() == std::io::ErrorKind::UnexpectedEof {
                #[cfg(feature = "tracing")]
                {
                    tracing::error!(target: "key_registry", "while reading proto buf in RegistryIterator::next");
                }
                e
            } else {
                #[cfg(feature = "tracing")]
                {
                    tracing::error!(target: "key_registry", err=%e);
                }
                e
            }
        })?;

        // Check checksum.
        if vpb::checksum::crc32fast::hash(&data)
            != u32::from_be_bytes(self.crc_buf[4..].try_into().unwrap())
        {
            return Err(Error::ChecksumMismatch);
        }

        let datakey = DataKey::unmarshal(&data).map_err(|e| {
            #[cfg(feature = "tracing")]
            {
                tracing::error!(target: "key_registry", err=%e, "while unmarshal of datakey in keyRegistryIterator.next");
            }
            e
        })?;
        // Decrypt the key if the storage key exists.
        datakey.data.encrypt_to_vec(self.encryption.secret(), datakey.iv.as_ref(), self.encryption.algorithm()).map(|v| {
            DataKey {
                key_id: datakey.key_id,
                iv: datakey.iv,
                data: v.into(),
                created_at: datakey.created_at,
            }
        }).map_err(|e| {
            #[cfg(feature = "tracing")]
            {
                tracing::error!(target: "key_registry", err=%e, "while decrypting datakey in keyRegistryIterator.next");
            }
            e.into()
        })
    }

    fn read_meta(file: &'a mut File, encryption: &'a Encryption) -> Result<&'a mut File> {
        if encryption.is_none() {
            let mut sanity = [0u8; SANITY_TEXT.len()];
            file.read_exact(&mut sanity)
                .map_err(From::from)
                .and_then(|_| {
                    if sanity.ne(SANITY_TEXT) {
                        Err(Error::SanityError)
                    } else {
                        Ok(file)
                    }
                })
        } else {
            let mut iv = [0u8; Encryption::BLOCK_SIZE];
            let mut e_sanity = [0u8; SANITY_TEXT.len()];
            file.read_exact(&mut iv)
                .and_then(|_| file.read_exact(&mut e_sanity))
                .map_err(From::from)
                .and_then(|_| {
                    e_sanity
                        .encrypt_to_vec(encryption.secret(), &iv, encryption.algorithm())
                        .map_err(From::from)
                        .and_then(|v| {
                            if v.eq(SANITY_TEXT) {
                                Ok(file)
                            } else {
                                Err(Error::SecrectMismatch)
                            }
                        })
                })
        }
    }
}

/// Return datakey if the last generated key duration less than
/// rotation duration.
#[inline]
fn valid_key(last_created: u64, duration: Duration) -> bool {
    // Time diffrence from the last generated time.
    let diff = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        - Duration::from_secs(last_created);
    diff < duration
}

#[inline]
fn store_data_key(buf: &mut BytesMut, mut dk: DataKey, opts: &RegistryOptions) -> Result<()> {
    dk.data
        .encrypt_to_vec(opts.secret(), dk.iv.as_ref(), opts.encryption.algorithm())
        .map_err(From::from)
        .map(|data| {
            dk.data = data.into();
            let data = dk.marshal();
            buf.put_u32(data.len() as u32);
            buf.put_u32(vpb::checksum::crc32fast::hash(&data));
            buf.put_slice(&data);
        })
}

#[cfg(test)]
pub(crate) mod test {
    use super::*;

    use rand::RngCore;
    use scopeguard::defer;
    use std::fs;
    use std::path::Path;

    pub(crate) fn get_registry_test_options<P: AsRef<Path>>(
        dir: P,
        encryption: Encryption,
    ) -> RefCounter<RegistryOptions> {
        RegistryOptions {
            dir: dir.as_ref().to_path_buf(),
            in_memory: false,
            read_only: false,
            encryption_key_rotation_duration: Default::default(),
            encryption,
        }
        .into()
    }

    fn get_memory_registry_test_options<P: AsRef<Path>>(
        dir: P,
        key: Vec<u8>,
    ) -> RefCounter<RegistryOptions> {
        RegistryOptions {
            dir: dir.as_ref().to_path_buf(),
            in_memory: true,
            read_only: false,
            encryption_key_rotation_duration: Default::default(),
            encryption: Encryption::aes(key),
        }
        .into()
    }

    #[test]
    fn test_build_registry() {
        let mut ek: [u8; 32] = Default::default();
        let mut rng = rand::thread_rng();
        rng.fill_bytes(&mut ek);

        let mut dir = std::env::temp_dir();
        dir.push("vela-test-build-registry");

        fs::create_dir(dir.clone()).unwrap();
        defer!(fs::remove_dir_all(&dir).unwrap(););

        let opt = get_registry_test_options(&dir, Encryption::aes(ek.to_vec()));

        let kr = Registry::open(opt.clone()).unwrap();
        let dk = kr.latest_data_key().unwrap();

        // We're resetting the last created timestamp. So, it creates
        // new datakey.
        match &kr {
            Registry::Memory(r) => {
                r.inner.write().last_created = 0;
            }
            Registry::Persistent(r) => {
                r.inner.write().last_created = 0;
            }
        }

        let dk1 = kr.latest_data_key().unwrap();
        // We generated two key. So, checking the length.
        assert_eq!(2, kr.num_data_keys());
        drop(kr);

        let kr2 = Registry::open(opt).unwrap();
        assert_eq!(2, kr2.num_data_keys());

        // Asserting the correctness of the datakey after opening the registry.
        assert_eq!(dk.data, kr2.data_key(&dk.key_id).unwrap().unwrap().data);
        assert_eq!(dk1.data, kr2.data_key(&dk1.key_id).unwrap().unwrap().data);
    }

    #[test]
    fn test_rewrite_registry() {
        let mut ek: [u8; 32] = Default::default();
        let mut rng = rand::thread_rng();
        rng.fill_bytes(&mut ek);

        let mut dir = std::env::temp_dir();
        dir.push("vela-test-rewrite-registry");

        fs::create_dir(dir.clone()).unwrap();
        defer!(fs::remove_dir_all(&dir).unwrap(););

        let opt = get_registry_test_options(&dir, Encryption::aes(ek.to_vec()));

        let kr = Registry::open(opt.clone()).unwrap();
        let _ = kr.latest_data_key().unwrap();
        // We're resetting the last created timestamp. So, it creates
        // new datakey.
        match &kr {
            Registry::Memory(r) => {
                r.inner.write().last_created = 0;
            }
            Registry::Persistent(r) => {
                r.inner.write().last_created = 0;
            }
        }
        let _ = kr.latest_data_key().unwrap();

        match &kr {
            Registry::Memory(r) => {
                r.inner.write().data_keys.remove(&1);
            }
            Registry::Persistent(r) => {
                r.inner.write().data_keys.remove(&1);
            }
        }

        kr.rewrite(opt.clone()).unwrap();
        drop(kr);

        let kr2 = Registry::open(opt).unwrap();
        assert_eq!(1, kr2.num_data_keys());
    }

    #[test]
    fn test_mismatch() {
        let mut ek: [u8; 32] = Default::default();
        let mut rng = rand::thread_rng();
        rng.fill_bytes(&mut ek);

        let mut dir = std::env::temp_dir();
        dir.push("vela-test-mismatch");

        fs::create_dir(dir.clone()).unwrap();
        defer!(fs::remove_dir_all(&dir).unwrap(););

        let opt = get_registry_test_options(&dir, Encryption::aes(ek.to_vec()));

        let kr = Registry::open(opt.clone()).unwrap();
        drop(kr);

        // Opening with the same key
        let _kr = Registry::open(opt).unwrap();

        // Opening with the invalid key
        let mut ek: [u8; 32] = Default::default();
        let mut rng = rand::thread_rng();
        rng.fill_bytes(&mut ek);
        let opt = get_registry_test_options(&dir, Encryption::aes(ek.to_vec()));

        assert!(Registry::open(opt).is_err());
    }

    #[test]
    fn test_encryption_decryption() {
        let mut ek: [u8; 32] = Default::default();
        let mut rng = rand::thread_rng();
        rng.fill_bytes(&mut ek);

        let mut dir = std::env::temp_dir();
        dir.push("vela-test-encryption-decryption");

        fs::create_dir(dir.clone()).unwrap();
        defer!(fs::remove_dir_all(&dir).unwrap(););

        let opt = get_registry_test_options(&dir, Encryption::aes(ek.to_vec()));

        let kr = Registry::open(opt.clone()).unwrap();

        let dk = kr.latest_data_key().unwrap();
        drop(kr);

        // Checking the correctness of the datakey after closing and
        // opening the key registry.
        let kr = Registry::open(opt).unwrap();
        let dk1 = kr.data_key(&dk.key_id).unwrap().unwrap();
        assert_eq!(dk.data, dk1.data);
    }

    #[test]
    fn test_key_registry_in_memory() {
        let mut ek: [u8; 32] = Default::default();
        let mut rng = rand::thread_rng();
        rng.fill_bytes(&mut ek);

        let opt = get_memory_registry_test_options("", ek.to_vec());

        let kr = Registry::open(opt).unwrap();
        let _ = kr.latest_data_key().unwrap();

        // We're resetting the last created timestamp. So, it creates
        // new datakey.
        match &kr {
            Registry::Memory(r) => {
                r.inner.write().last_created = 0;
            }
            Registry::Persistent(r) => {
                r.inner.write().last_created = 0;
            }
        }

        let _ = kr.latest_data_key().unwrap();
        // We generated two key. So, checking the length.
        assert_eq!(2, kr.num_data_keys());
    }
}
