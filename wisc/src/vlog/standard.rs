use std::path::PathBuf;

use super::VALUE_LOG_FILE_EXTENSION;

mod discard;
use discard::*;

pub struct ValueLogOptions {
    pub size: u64,
    pub in_memory: bool,
    pub dir_path: PathBuf,
}

pub struct ValueLog {
    dir_path: PathBuf,

    // guards our view of which files exist, which to be deleted, how many active iterators
    max_fid: u32,

    opts: ValueLogOptions,
}

impl ValueLog {
    #[inline]
    fn fpath(&self, fid: u32) -> PathBuf {
        Self::file_path(&self.dir_path, fid)
    }

    #[inline]
    fn file_path(dir_path: &PathBuf, max_fid: u32) -> PathBuf {
        let mut path = dir_path.join(format!("{:06}", max_fid));
        path.set_extension(VALUE_LOG_FILE_EXTENSION);
        path
    }
}
