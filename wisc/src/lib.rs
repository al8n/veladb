#![cfg_attr(not(feature = "std"), no_std)]

pub mod error;

mod registry;
pub use registry::*;

mod wal;
pub use wal::*;

mod vlog;
pub use vlog::*;

pub mod metrics;

bitflags::bitflags! {
    /// Values have their first byte being byteData or byteDelete. This helps us distinguish between
    /// a key that has never been seen and a key that has been explicitly deleted.
    pub struct OP: u8 {
        #[doc = "Set if the key has been deleted."]
        const BIT_DELETE = 1 << 0;
        #[doc = "Set if the value is NOT stored directly next to key."]
        const BIT_VALUE_POINTER = 1 << 1;
        #[doc = "Set if earlier versions can be discarded."]
        const BIT_DISCARD_EARLIER_VERSIONS = 1 << 2;
        #[doc = "Set if item shouldn't be discarded via compactions (used by merge operator)"]
        const BIT_MERGE_ENTRY = 1 << 3;
        #[doc = "Set if the entry is part of a txn."]
        const BIT_TXN = 1 << 6;
        #[doc = "Set if the entry is to indicate end of txn in value log."]
        const BIT_FIN_TXN = 1 << 7;
    }
}

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
