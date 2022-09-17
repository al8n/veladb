#![cfg_attr(not(feature = "std"), no_std)]

pub mod error;

mod registry;
pub use registry::*;

/// open_trunc_file creates the file with O_RDWR | O_CREATE | O_TRUNC
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

/// open_trunc_file creates the file with O_RDWR | O_CREATE | O_TRUNC
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
