#![cfg_attr(not(feature = "std"), no_std)]

pub mod error;

mod registry;
use core::cell::Cell;

pub use registry::*;

mod wal;
use vpb::kvstructs::{EntryRef, Header, KeyExt, ValueRef};
pub use wal::*;

mod vlog;
pub use vlog::*;

mod mem_table;
pub use mem_table::*;

pub mod metrics;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[repr(C)]
pub struct ValuePointer {
    pub fid: u32,
    pub len: u32,
    pub offset: u32,
}

const VP_SIZE: usize = core::mem::size_of::<ValuePointer>();

impl ValuePointer {
    /// Encodes Pointer into `[u8; 12]`.
    pub fn encode(&self) -> [u8; VP_SIZE] {
        let mut data = [0u8; VP_SIZE];
        data[..4].copy_from_slice(u32::to_be_bytes(self.fid).as_ref());
        data[4..8].copy_from_slice(u32::to_be_bytes(self.len).as_ref());
        data[8..].copy_from_slice(u32::to_be_bytes(self.offset).as_ref());
        data
    }

    #[inline]
    pub fn is_zero(&self) -> bool {
        self.fid == 0 && self.offset == 0 && self.len == 0
    }

    /// Decodes the value pointer into the provided byte buffer.
    #[inline]
    pub fn decode(data: &[u8]) -> Self {
        let fid = u32::from_be_bytes(data[..4].try_into().unwrap());
        let len = u32::from_be_bytes(data[4..8].try_into().unwrap());
        let offset = u32::from_be_bytes(data[8..].try_into().unwrap());
        Self { fid, len, offset }
    }
}

impl PartialOrd<Self> for ValuePointer {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ValuePointer {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        if self.fid != other.fid {
            return self.fid.cmp(&other.fid);
        }

        if self.offset != other.offset {
            return self.offset.cmp(&other.offset);
        }

        self.len.cmp(&other.len)
    }
}

struct HashReader<'a, R> {
    r: &'a mut R,
    h: vpb::checksum::crc32fast::Hasher,
    byte_read: usize,
}

#[cfg(feature = "std")]
impl<'a, R: std::io::Read> std::io::Read for HashReader<'a, R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.r.read(buf).map(|n| {
            self.h.update(&buf[..n]);
            self.byte_read += n;
            n
        })
    }
}

impl<'a, R> HashReader<'a, R> {
    fn new(r: &'a mut R) -> Self {
        Self {
            r,
            h: vpb::checksum::crc32fast::Hasher::new(),
            byte_read: 0,
        }
    }

    fn checksum(self) -> u32 {
        self.h.finalize()
    }
}

struct SafeRead {
    kv: *mut Vec<u8>,
    record_offset: Cell<u32>,
}

impl SafeRead {
    /// Read from the read buffer, return the entry and num bytes read.
    #[cfg(feature = "std")]
    fn read_entry(
        &self,
        reader: &[u8],
        base_iv: &[u8],
        secret: Option<&[u8]>,
        algo: vpb::EncryptionAlgorithm,
    ) -> crate::error::Result<(usize, vpb::kvstructs::EntryRef<'_>)> {
        use std::io::Read;

        let mut read = 0;
        let mut reader = std::io::Cursor::new(reader);
        let mut tee = HashReader::new(&mut reader);
        let (h_size, h) = Header::decode_from_reader(&mut tee)?;
        read += h_size;

        let kl = h.get_key_len() as usize;
        if kl > (1 << 16) {
            return Err(crate::error::Error::Truncate);
        }

        let vl = h.get_value_len() as usize;
        let kv = unsafe { &mut *self.kv };
        if kv.len() < vl + kl {
            kv.resize(vl + kl, 0);
        }

        read += kl + vl;
        if let Err(e) = tee.read_exact(kv) {
            if e.kind() == std::io::ErrorKind::UnexpectedEof {
                return Err(crate::error::Error::Truncate);
            }
        }

        let record_offset = self.record_offset.get();
        if secret.is_some() {
            let _ = core::mem::replace(
                kv,
                WAL::decrypt_kv(kv, secret, base_iv, record_offset, algo)?,
            );
        }

        let kr = kv[..kl].as_key_ref();
        let vr = ValueRef::new(
            h.get_meta(),
            h.get_user_meta(),
            h.get_expires_at(),
            0,
            &kv[kl..],
        );

        let mut crc_buf = [0; core::mem::size_of::<u32>()];
        read += crc_buf.len();
        if let Err(e) = tee.r.read_exact(&mut crc_buf) {
            if e.kind() == std::io::ErrorKind::UnexpectedEof {
                return Err(crate::error::Error::Truncate);
            }
        }
        let crc = u32::from_be_bytes(crc_buf);
        if crc != tee.checksum() {
            return Err(crate::error::Error::Truncate);
        }

        let vplen = (h_size + kv.len() + core::mem::size_of::<u32>()) as u32;
        self.record_offset.set(record_offset + vplen);
        Ok((read, EntryRef::new(kr, vr, record_offset, h_size, 0)))
    }
}

/// Open or create file with `O_RDWR | O_CREATE | O_TRUNC`
#[cfg(all(feature = "std", unix))]
#[inline(always)]
fn open_trunc_file<P: AsRef<std::path::Path>>(
    filename: P,
    sync: bool,
) -> std::io::Result<std::fs::File> {
    use rustix::fs::OFlags;
    use std::os::unix::fs::OpenOptionsExt;
    let mut flags = OFlags::RDWR | OFlags::CREATE | OFlags::TRUNC;

    if sync {
        flags |= OFlags::DSYNC;
    }

    std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .custom_flags(flags.bits())
        .mode(0o600)
        .open(filename)
}

/// Open or create file with `O_RDWR | O_CREATE | O_TRUNC`
#[cfg(all(feature = "std", windows))]
#[inline(always)]
fn open_trunc_file<P: AsRef<std::path::Path>>(
    filename: P,
    sync: bool,
) -> std::io::Result<std::fs::File> {
    use rustix::fs::OFlags;
    use std::os::windows::fs::OpenOptionsExt;
    let mut flags = OFlags::RDWR | OFlags::CREATE | OFlags::TRUNC;

    if sync {
        flags |= OFlags::DSYNC;
    }

    std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .custom_flags(flags.bits() as u32)
        .access_mode(0o600)
        .open(filename)
}

#[cfg(feature = "std")]
pub fn is_deleted_or_expired(meta: u8, expires_at: u64) -> bool {
    if (meta & bable::kvstructs::OP::BIT_DELETE.bits()) > 0 {
        return true;
    }

    if expires_at == 0 {
        return false;
    }

    expires_at
        <= std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
}
