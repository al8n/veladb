use crate::error::*;
use fmmap::{MmapFileExt, MmapFileMut, MmapFileMutExt, Options};
use indexsort::IndexSort;
use parking_lot::Mutex;
use vela_utils::ref_counter::RefCounter;

const DISCARD_FILE_NAME: &'static str = "DISCARD";
const DISCARD_FILE_EXTENSION: &'static str = "discard";

/// Keeps track of the amount of data that could be discarded for
/// a given logfile.
#[derive(Clone)]
#[repr(transparent)]
pub struct DiscardStats {
    inner: RefCounter<Mutex<Inner>>,
}

struct Inner {
    mmap: MmapFileMut,
    next_empty_slot: usize,
}

impl IndexSort for Inner {
    fn len(&self) -> usize {
        self.next_empty_slot
    }

    fn less(&self, i: usize, j: usize) -> bool {
        self.get(16 * i) < self.get(16 * j)
    }

    fn swap(&mut self, i: usize, j: usize) {
        let src_ptr = self.mmap.as_mut_slice().as_mut_ptr();
        let mut tmp = [0; 16];

        unsafe {
            let left_ptr = src_ptr.add(16 * i);
            let right_ptr = src_ptr.add(16 * j);
            core::ptr::copy_nonoverlapping(left_ptr, tmp.as_mut_ptr(), 16);
            core::ptr::copy_nonoverlapping(right_ptr, left_ptr, 16);
            core::ptr::copy_nonoverlapping(tmp.as_ptr(), right_ptr, 16);
        }
    }
}

impl Inner {
    #[inline]
    fn new(mmap: MmapFileMut, exists: bool) -> Self {
        let mut this = Self {
            mmap,
            next_empty_slot: 0,
        };

        if !exists {
            this.zero_out();
        }

        for slot in 0..this.max_slot() {
            if this.get(16 * slot) == 0 {
                this.next_empty_slot = slot;
                break;
            }
        }

        this.sort();

        #[cfg(feature = "tracing")]
        {
            tracing::info!(
                target: "discard_stats",
                "discard stats next_empty_slot: {}",
                this.next_empty_slot
            );
        }

        this
    }

    #[inline]
    fn set(&mut self, offset: usize, val: u64) {
        self.mmap
            .slice_mut(offset, offset + 8)
            .copy_from_slice(&val.to_be_bytes());
    }

    #[inline]
    fn get(&self, offset: usize) -> u64 {
        u64::from_be_bytes(self.mmap.slice(offset, offset + 8).try_into().unwrap())
    }

    /// Zero out the next slot
    #[inline]
    fn zero_out(&mut self) {
        self.set(self.next_empty_slot * 16, 0);
        self.set(self.next_empty_slot * 16 + 8, 0)
    }

    #[inline]
    fn max_slot(&self) -> usize {
        self.mmap.len() / 16
    }

    #[inline]
    fn iterate<F: FnMut(u64, u64)>(&mut self, mut f: F) {
        for slot in 0..self.next_empty_slot {
            let idx = 16 * slot;
            f(self.get(idx), self.get(idx + 8))
        }
    }
}

impl DiscardStats {
    pub fn new(value_dir: impl AsRef<std::path::Path>) -> Result<Self> {
        let mut fname = value_dir.as_ref().join(DISCARD_FILE_NAME);
        fname.set_extension(DISCARD_FILE_EXTENSION);

        const DISCARD_FILE_MAX_SIZE: u64 = 1 << 20; // 1GB

        Options::new()
            .read(true)
            .create(true)
            .write(true)
            .max_size(DISCARD_FILE_MAX_SIZE)
            .open_mmap_file_mut(&fname)
            .map(|mmap| Self::new_in(mmap, fname.exists()))
            .map_err(From::from)
    }

    #[inline(always)]
    fn new_in(mmap: MmapFileMut, exists: bool) -> Self {
        Self {
            inner: RefCounter::new(Mutex::new(Inner::new(mmap, exists))),
        }
    }

    /// Update the discard stats for the given file id. If discard is
    /// 0, it would return the current value of discard for the file. If discard is
    /// < 0, it would set the current value of discard to zero for the file.
    pub fn update(&self, fidu: u32, discard: i64) -> i64 {
        let fid = fidu as u64;
        let mut inner = self.inner.lock();
        let mut idx = indexsort::search(inner.next_empty_slot, |slot| inner.get(16 * slot) >= fid);

        if idx < inner.next_empty_slot && inner.get(16 * idx) == fid {
            let off = idx * 16 + 8;
            let cur_disc = inner.get(off);
            if discard == 0 {
                return cur_disc as i64;
            }

            if discard < 0 {
                inner.set(off, 0);
                return 0;
            }

            inner.set(off, cur_disc + discard as u64);
            return (cur_disc + discard as u64) as i64;
        }

        if discard <= 0 {
            // No need to add a new entry.
            return 0;
        }

        // Could not find the fid. Add the entry.
        idx = inner.next_empty_slot;
        inner.set(idx * 16, fid);
        inner.set(idx * 16 + 8, discard as u64);

        // Move to next slot.
        inner.next_empty_slot += 1;
        while inner.next_empty_slot >= inner.max_slot() {
            let cap = inner.mmap.len() as u64;
            inner.mmap.truncate(2 * cap).unwrap();
        }
        inner.zero_out();
        inner.sort();
        discard
    }

    pub fn iterate<F: FnMut(u64, u64)>(&self, f: F) {
        self.inner.lock().iterate(f)
    }

    /// Returns the file id with maximum discard bytes
    pub fn max_discard(&self) -> (u32, u64) {
        let mut inner = self.inner.lock();
        let mut max_fid = 0u64;
        let mut max_val = 0;
        inner.iterate(|fid, val| {
            if max_val < val {
                max_fid = fid;
                max_val = val;
            }
        });
        (max_fid as u32, max_val)
    }
}
