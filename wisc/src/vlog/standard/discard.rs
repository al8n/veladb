use fmmap::{MmapFileExt, MmapFileMut, MmapFileMutExt};
use parking_lot::Mutex;
use vela_utils::{binary_search, binary_search_by_usize, ref_counter::RefCounter};

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

impl Inner {
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
    /// Update the discard stats for the given file id. If discard is
    /// 0, it would return the current value of discard for the file. If discard is
    /// < 0, it would set the current value of discard to zero for the file.
    pub fn update(&self, fidu: u32, discard: i64) -> i64 {
        let fid = fidu as u64;
        let mut inner = self.inner.lock();
        let mut idx =
            binary_search_by_usize(inner.next_empty_slot, |slot| inner.get(16 * slot) >= fid);

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

        let mut data = inner.mmap.as_mut_slice();

        data.sort_unstable_by(|a, b| {});
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
