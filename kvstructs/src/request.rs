use crate::{Entry, ValuePointer, MAX_HEADER_SIZE};
use wg::WaitGroup;

/// A request contains multiple entries to be written into LSM tree.
#[derive(Default, Clone)]
pub struct Request {
    /// Entries contained in this request
    pub entries: Vec<Entry>,
    /// Offset in vLog (will be updated upon processing the request)
    pub ptrs: Vec<ValuePointer>,
    /// Use channel to notify that the value has been persisted to disk
    pub wg: WaitGroup,
}

impl Request {
    /// Returns the size that needed to be written for the given request.
    #[inline]
    pub fn estimated_size(&self) -> u64 {
        self.entries
            .iter()
            .map(|e| {
                (MAX_HEADER_SIZE + e.key.len() + e.val.len()) as u64
                    + core::mem::size_of::<u32>() as u64
            })
            .sum()
    }
}
