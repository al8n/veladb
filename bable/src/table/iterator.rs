use super::*;
use crate::{Header, HEADER_SIZE};
use alloc::boxed::Box;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::iter::FusedIterator;
use vela_utils::binary_search;
use vpb::kvstructs::{
    bytes::{Bytes, BytesMut},
    iterator::SeekFrom,
    KeyExt, KeyRef, ValueExt, ValueRef,
};

bitflags::bitflags! {
    pub struct Flag: u8 {
        const NONE = 0;
        const REVERSED = 2;
        const NO_CACHE = 4;
    }
}

pub trait BableIterator {
    fn next(&mut self);
    fn rewind(&mut self);
    /// seek will reset iterator and seek to >= key.
    fn seek(&mut self, key: impl KeyExt);
    fn entry(&self) -> Option<(KeyRef, ValueRef)>;
    fn key(&self) -> Option<KeyRef>;
    fn val(&self) -> Option<ValueRef>;
    fn valid(&self) -> bool;
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, None)
    }
    #[inline]
    fn count(&self) -> usize
    where
        Self: Sized,
    {
        match self.size_hint().1 {
            None => usize::MAX,
            Some(v) => v,
        }
    }
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub enum IterError {
    EOF,
    Other(String),
}

impl core::fmt::Display for IterError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            IterError::EOF => write!(f, "EOF"),
            IterError::Other(s) => write!(f, "{}", s),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for IterError {}

pub(crate) struct BlockIter {
    pub(crate) data: Bytes,
    pub(crate) idx: isize,
    pub(crate) err: Option<IterError>,
    pub(crate) base_key: Bytes,
    pub(crate) key: BytesMut,
    pub(crate) val: Bytes,
    pub(crate) block: RefCounter<Block>,
    pub(crate) block_id: isize,
    pub(crate) table_id: u64,
    // prev_overlap stores the overlap of the previous key with the base key.
    // This avoids unnecessary copy of base key when the overlap is same for multiple keys.
    pub(crate) prev_overlap: u16,
}

impl BlockIter {
    #[inline]
    pub(crate) fn new(block: RefCounter<Block>, table_id: u64, block_id: isize) -> Self {
        Self {
            data: block.data(),
            idx: 0,
            err: None,
            base_key: Default::default(),
            key: Default::default(),
            val: Default::default(),
            block,
            block_id,
            table_id,
            prev_overlap: 0,
        }
    }

    #[inline]
    pub fn get_key(&self) -> KeyRef {
        KeyRef::new(self.key.as_ref())
    }

    #[inline]
    pub fn get_val(&self) -> ValueRef {
        ValueRef::decode_value_ref(self.val.as_ref())
    }

    #[inline(always)]
    pub fn entry(&self) -> (KeyRef, ValueRef) {
        // shallow clone happens here, which incrs the reference
        // counter and does not copy all of the data
        (
            KeyRef::new(self.key.as_ref()),
            ValueRef::decode_value_ref(self.val.as_ref()),
        )
    }

    pub fn set_block(&mut self, block: RefCounter<Block>) {
        // Decrement the ref for the old block. If the old block was compressed, we
        // might be able to reuse it.

        self.err = None;
        self.idx = 0;
        self.base_key.clear();
        self.prev_overlap = 0;
        self.key.clear();
        self.val.clear();

        // Drop the index from the block. We don't need it anymore.
        self.data = block.data();
        self.block = block;
    }

    #[inline(always)]
    pub fn valid(&self) -> bool {
        self.err.is_none()
    }

    /// seek brings us to the first block element that is >= input key.
    pub(crate) fn seek(&mut self, key: &[u8], whence: SeekFrom) {
        self.err = None;
        let start_index = match whence {
            // We don't need to do anything. startIndex is already at 0
            SeekFrom::Origin => 0,
            SeekFrom::Current => self.idx,
        }; // This tells from which index we should start binary search.

        //TODO: use Rust binary search
        let found_entry_index = binary_search(self.block.entry_offsets.len() as isize, |idx| {
            // If idx is less than start index then just return false.
            if idx < start_index {
                false
            } else {
                self.set_idx(idx);
                match self.key.as_key_ref().compare_key(key) {
                    core::cmp::Ordering::Less => false,
                    core::cmp::Ordering::Equal | core::cmp::Ordering::Greater => true,
                }
            }
        });

        self.set_idx(found_entry_index)
    }

    /// seek_to_first brings us to the first element.
    pub(crate) fn seek_to_first(&mut self) {
        self.set_idx(0)
    }

    /// seek_to_last brings us to the last element.
    pub(crate) fn seek_to_last(&mut self) {
        self.set_idx(self.block.entry_offsets.len() as isize - 1)
    }

    pub(crate) fn prev(&mut self) {
        if self.idx == 0 {
            self.err = Some(IterError::EOF);
        } else {
            self.set_idx(self.idx - 1)
        }
    }

    #[inline]
    pub(crate) fn seek_to_block(
        &mut self,
        block: RefCounter<Block>,
        table_id: u64,
        block_id: isize,
        prev: bool,
    ) {
        self.table_id = table_id;
        self.block_id = block_id;
        self.set_block(block);
        if prev {
            self.seek_to_last();
        } else {
            self.seek_to_first();
        }
    }

    /// set_idx sets the iterator to the entry at index i and set it's key and value.
    #[inline]
    pub(crate) fn set_idx(&mut self, idx: isize) {
        self.idx = idx;
        let block_entry_offsets_len = self.block.entry_offsets.len() as isize;
        if self.idx >= block_entry_offsets_len as isize || self.idx < 0 {
            self.err = Some(IterError::EOF);
            return;
        }
        self.err = None;
        let idx = idx as usize;
        let start_offset = self.block.entry_offsets[idx] as usize;

        // set base key
        if self.base_key.is_empty() {
            let h = Header::decode(self.data.as_ref());
            self.base_key = self
                .data
                .slice(HEADER_SIZE..HEADER_SIZE + h.diff() as usize);
        }

        let end_offset = if self.idx + 1 == block_entry_offsets_len {
            self.data.len()
        } else {
            // idx point to some entry other than the last one in the block.
            // EndOffset of the current entry is the start offset of the next entry.
            self.block.entry_offsets[idx + 1] as usize
        };

        let entry_data = self.data.slice(start_offset..end_offset);
        let h = Header::decode(entry_data.as_ref());
        // Header contains the length of key overlap and difference compared to the base key. If the key
        // before this one had the same or better key overlap, we can avoid copying that part into
        // itr.key. But, if the overlap was lesser, we could copy over just that portion.
        if h.overlap() > self.prev_overlap {
            let prev_overlap_len = self.prev_overlap as usize;
            let header_overlap_len = h.overlap() as usize;
            self.key.truncate(prev_overlap_len);
            self.key
                .extend_from_slice(&self.base_key[prev_overlap_len..header_overlap_len]);
        }
        self.prev_overlap = h.overlap();
        let val_offset = HEADER_SIZE + h.diff() as usize;
        let diff_key = &entry_data[HEADER_SIZE..val_offset];
        self.key.truncate(h.overlap() as usize);
        self.key.extend_from_slice(diff_key);
        self.val = entry_data.slice(val_offset..);

        scopeguard::defer_on_unwind!(
            //TODO: log
            let mut debug_buf = String::new();

            debug_buf.push_str("==== Recovered ====\n");
            debug_buf.push_str(format!("Table ID: {}\nBlock Idx: {}\nEntry Idx: {}\nData len: {}\nStartOffset: {}\nEndOffset: {}\nEntryOffsets len: {}\nEntryOffsets: {:?}\n", self.table_id, self.block_id, self.idx, self.data.len(), start_offset, end_offset, self.block.entry_offsets.len(), self.block.entry_offsets).as_str());
            #[cfg(feature = "tracing")]
            tracing::error!(target: "table_iterator", "{}", debug_buf);
        );
    }
}

impl core::iter::Iterator for BlockIter {
    type Item = RefCounter<Block>;

    fn next(&mut self) -> Option<Self::Item> {
        self.nth((self.idx + 1) as usize)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.block.entry_offsets.len()))
    }

    fn count(self) -> usize
    where
        Self: Sized,
    {
        self.block.entry_offsets.len()
    }

    fn last(mut self) -> Option<Self::Item>
    where
        Self: Sized,
    {
        self.nth(self.block.entry_offsets.len() - 1)
    }

    fn nth(&mut self, idx: usize) -> Option<Self::Item> {
        self.set_idx(idx as isize);
        match &self.err {
            None => Some(self.block.clone()),
            Some(_) => None,
        }
    }
}

impl ExactSizeIterator for BlockIter {}

impl FusedIterator for BlockIter {}

pub enum TableIterator {
    Concat(ConcatTableIterator),
    Merge(MergeTableIterator),
    Uni(UniTableIterator<RefCounter<RawTable>>),
}

impl TableIterator {
    pub fn next(&mut self) {
        match self {
            TableIterator::Uni(iter) => iter.next(),
            TableIterator::Merge(iter) => iter.next(),
            TableIterator::Concat(iter) => iter.next(),
        }
    }

    pub fn rewind(&mut self) {
        match self {
            TableIterator::Uni(iter) => iter.rewind(),
            TableIterator::Merge(iter) => iter.rewind(),
            TableIterator::Concat(iter) => iter.rewind(),
        }
    }

    pub fn seek(&mut self, key: impl KeyExt) {
        match self {
            TableIterator::Uni(iter) => iter.seek(key),
            TableIterator::Merge(iter) => iter.seek(key),
            TableIterator::Concat(iter) => iter.seek(key),
        }
    }

    pub fn entry(&self) -> Option<(KeyRef, ValueRef)> {
        match self {
            TableIterator::Uni(iter) => iter.entry(),
            TableIterator::Merge(iter) => iter.entry(),
            TableIterator::Concat(iter) => iter.entry(),
        }
    }

    pub fn key(&self) -> Option<KeyRef> {
        match self {
            TableIterator::Uni(iter) => iter.key(),
            TableIterator::Merge(iter) => iter.key(),
            TableIterator::Concat(iter) => iter.key(),
        }
    }

    pub fn val(&self) -> Option<ValueRef> {
        match self {
            TableIterator::Uni(iter) => iter.val(),
            TableIterator::Merge(iter) => iter.val(),
            TableIterator::Concat(iter) => iter.val(),
        }
    }

    pub fn valid(&self) -> bool {
        match self {
            TableIterator::Uni(iter) => iter.valid(),
            TableIterator::Merge(iter) => iter.valid(),
            TableIterator::Concat(iter) => iter.valid(),
        }
    }

    pub fn count(&self) -> usize
    where
        Self: Sized,
    {
        match self {
            TableIterator::Concat(iter) => iter.count(),
            TableIterator::Merge(iter) => iter.count(),
            TableIterator::Uni(iter) => iter.count(),
        }
    }

    pub fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            TableIterator::Concat(iter) => iter.size_hint(),
            TableIterator::Merge(iter) => iter.size_hint(),
            TableIterator::Uni(iter) => iter.size_hint(),
        }
    }
}

impl From<UniTableIterator<RefCounter<RawTable>>> for TableIterator {
    fn from(iter: UniTableIterator<RefCounter<RawTable>>) -> Self {
        Self::Uni(iter)
    }
}

impl From<ConcatTableIterator> for TableIterator {
    fn from(iter: ConcatTableIterator) -> Self {
        Self::Concat(iter)
    }
}

impl From<MergeTableIterator> for TableIterator {
    fn from(iter: MergeTableIterator) -> Self {
        Self::Merge(iter)
    }
}

pub struct UniTableIterator<I: AsRef<RawTable>> {
    table: I,
    bpos: isize,
    bi: Option<BlockIter>,
    err: Option<IterError>,
    num_entries: usize,

    // Internally, Iterator is bidirectional. However, we only expose the
    // unidirectional functionality for now.
    opt: Flag, // Valid options are REVERSED and NOCACHE.
}

impl<I: AsRef<RawTable>> UniTableIterator<I> {
    pub(crate) fn new(t: I, opt: Flag) -> Self {
        let num_entries = t.as_ref().num_entries();
        Self {
            table: t,
            bpos: -1,
            bi: None,
            err: None,
            num_entries,
            opt,
        }
    }

    #[inline(always)]
    pub(crate) fn get_table_id(&self) -> u64 {
        self.table.as_ref().id()
    }

    #[inline]
    fn seek_helper_block_handle(
        &mut self,
        block: Result<RefCounter<Block>>,
        table_id: u64,
        key: &[u8],
    ) {
        match block {
            Ok(block) => {
                if let Some(ref mut bi) = self.bi {
                    bi.table_id = table_id;
                    bi.block_id = self.bpos;
                    bi.set_block(block);
                    bi.seek(key, SeekFrom::Origin);
                    self.err = bi.err.clone()
                } else {
                    let mut bi = BlockIter::new(block, self.table.as_ref().id(), self.bpos);
                    bi.seek(key, SeekFrom::Origin);
                    self.err = bi.err.clone();
                    self.bi = Some(bi);
                }
            }
            Err(e) => self.err = Some(IterError::Other(e.to_string())),
        }
    }

    #[inline]
    fn get_seek_position(&mut self, key: &[u8], from: SeekFrom) -> isize {
        self.err = None;
        match from {
            SeekFrom::Origin => self.reset(),
            SeekFrom::Current => {}
        }

        let index = self.table.as_ref().fetch_index();

        // Offsets should never return None since we're iterating within the OffsetsLength.
        binary_search(self.table.as_ref().offsets_length() as isize, |idx| {
            let block_offset = &index.offsets[idx as usize];
            match block_offset.key.as_key_ref().compare_key(key) {
                core::cmp::Ordering::Less | core::cmp::Ordering::Equal => false,
                core::cmp::Ordering::Greater => true,
            }
        })
    }

    #[inline]
    fn seek_to_first_block_handle(&mut self, block: Result<RefCounter<Block>>, table_id: u64) {
        match block {
            Ok(block) => match self.bi {
                None => {
                    let mut bi = BlockIter::new(block, self.get_table_id(), self.bpos);
                    bi.seek_to_first();
                    self.err = bi.err.clone();
                    self.bi = Some(bi);
                }
                Some(ref mut bi) => {
                    bi.table_id = table_id;
                    bi.block_id = self.bpos;
                    bi.set_block(block);
                    bi.seek_to_first();
                    self.err = bi.err.clone();
                }
            },
            Err(e) => self.err = Some(IterError::Other(e.to_string())),
        }
    }

    #[inline]
    fn seek_to_last_block_handle(&mut self, block: Result<RefCounter<Block>>, table_id: u64) {
        match block {
            Ok(block) => match self.bi {
                None => {
                    let mut bi = BlockIter::new(block, table_id, self.bpos);
                    bi.seek_to_last();
                    self.err = bi.err.clone();
                    self.bi = Some(bi);
                }
                Some(ref mut bi) => {
                    bi.table_id = table_id;
                    bi.block_id = self.bpos;
                    bi.set_block(block);
                    bi.seek_to_last();
                    self.err = bi.err.clone();
                }
            },
            Err(e) => self.err = Some(IterError::Other(e.to_string())),
        }
    }

    #[inline(always)]
    pub fn reset(&mut self) {
        self.bpos = 0;
        self.err = None;
    }

    #[inline(always)]
    fn use_cache(&self) -> bool {
        (self.opt & Flag::NO_CACHE).bits == 0
    }

    /// seek_from_key brings us to a key that is >= input key.
    fn seek_from_key(&mut self, key: &[u8], from: SeekFrom) {
        // Offsets should never return None since we're iterating within the OffsetsLength.
        let idx = self.get_seek_position(key, from);
        if idx == 0 {
            // The smallest key in our table is already strictly > key. We can return that.
            // This is like a SeekToFirst.
            self.seek_helper(0, key);
            return;
        }

        // block[idx].smallest is > key.
        // Since idx>0, we know block[idx-1].smallest is <= key.
        // There are two cases.
        // 1) Everything in block[idx-1] is strictly < key. In this case, we should go to the first
        //    element of block[idx].
        // 2) Some element in block[idx-1] is >= key. We should go to that element.
        self.seek_helper(idx - 1, key);
        if let Some(err) = self.err.as_ref() {
            match err {
                IterError::EOF => {
                    // Case 1. Need to visit block[idx].
                    if idx == self.table.as_ref().offsets_length() as isize {
                        // If idx == self.table.block_index.len(), then input key is greater than ANY element of table.
                        // There's nothing we can do. `valid()` should return false as we seek to end of table.
                    } else {
                        // Since block[idx].smallest is > key. This is essentially a block[idx].SeekToFirst.
                        self.seek_helper(idx, key);
                    }
                }
                IterError::Other(_) => {
                    // Case 2: No need to do anything. We already did the seek in block[idx-1].
                }
            }
        }
    }

    #[inline]
    fn seek_helper(&mut self, block_idx: isize, key: &[u8]) {
        self.bpos = block_idx;
        let block = self.table.as_ref().block(block_idx, self.use_cache());
        let table_id = self.get_table_id();
        self.seek_helper_block_handle(block, table_id, key)
    }

    fn next_in(&mut self) {
        self.err = None;
        if self.bpos >= self.table.as_ref().offsets_length() as isize {
            self.err = Some(IterError::EOF);
            return;
        }

        let use_cache = self.use_cache();
        let table_id = self.get_table_id();
        match self.bi {
            Some(ref mut bi) => {
                if bi.data.is_empty() {
                    let block = self.table.as_ref().block(self.bpos, use_cache);
                    match block {
                        Ok(block) => {
                            bi.seek_to_block(block, table_id, self.bpos, false);
                            self.err = bi.err.clone();
                        }
                        Err(e) => {
                            self.err = Some(IterError::Other(e.to_string()));
                        }
                    }
                } else {
                    bi.next();
                    if !bi.valid() {
                        self.bpos += 1;
                        bi.data.clear();
                        self.next_in();
                    }
                }
            }
            None => {
                self.err = Some(IterError::EOF);
            }
        }
    }

    pub fn prev(&mut self) {
        self.err = None;
        if self.bpos < 0 {
            self.err = Some(IterError::EOF);
            return;
        }

        let use_cache = self.use_cache();
        let table_id = self.get_table_id();

        if let Some(ref mut bi) = self.bi {
            if bi.data.is_empty() {
                let block = self.table.as_ref().block(self.bpos, use_cache);
                match block {
                    Ok(block) => {
                        bi.seek_to_block(block, table_id, self.bpos, true);
                        self.err = bi.err.clone();
                    }
                    Err(e) => {
                        self.err = Some(IterError::Other(e.to_string()));
                    }
                }
            } else {
                bi.prev();
                if !bi.valid() {
                    self.bpos = self.bpos.overflowing_sub(1).0;
                    bi.data.clear();
                    self.prev();
                }
            }
        }
    }

    /// seek_to_key will reset iterator and seek to <= input key.
    pub fn seek_to_key<K: KeyExt>(&mut self, key: K) {
        self.seek_from_key(key.as_bytes(), SeekFrom::Origin);
        if let Some(k) = self.key() {
            if k.as_slice().ne(key.as_bytes()) {
                self.prev();
            }
        }
    }

    pub fn seek_to_first(&mut self) {
        let num_blocks = self.table.as_ref().offsets_length();
        if num_blocks == 0 {
            self.err = Some(IterError::EOF);
            return;
        }

        self.bpos = 0;
        let block = self.table.as_ref().block(self.bpos, self.use_cache());
        let table_id = self.get_table_id();
        self.seek_to_first_block_handle(block, table_id)
    }

    pub fn seek_to_last(&mut self) {
        let num_blocks = self.table.as_ref().offsets_length() as isize;
        if num_blocks == 0 {
            self.err = Some(IterError::EOF);
            return;
        }

        self.bpos = num_blocks - 1;
        let block = self.table.as_ref().block(self.bpos, self.use_cache());
        let table_id = self.get_table_id();
        self.seek_to_last_block_handle(block, table_id)
    }
}

impl<I: AsRef<RawTable>> BableIterator for UniTableIterator<I> {
    fn next(&mut self) {
        if self.bpos == -1 {
            self.rewind();
        } else if (self.opt & Flag::REVERSED).bits() == 0 {
            self.next_in();
        } else {
            self.prev();
        }
    }

    fn rewind(&mut self) {
        if (self.opt & Flag::REVERSED).bits() == 0 {
            self.seek_to_first()
        } else {
            self.seek_to_last()
        }
    }

    fn seek(&mut self, key: impl KeyExt) {
        if (self.opt & Flag::REVERSED).bits() == 0 {
            self.seek_from_key(key.as_bytes(), SeekFrom::Origin)
        } else {
            self.seek_to_key(key)
        }
    }

    fn entry(&self) -> Option<(KeyRef, ValueRef)> {
        self.bi.as_ref().map(|bi| bi.entry())
    }

    fn key(&self) -> Option<KeyRef> {
        self.bi.as_ref().map(|bi| bi.get_key())
    }

    fn val(&self) -> Option<ValueRef> {
        self.bi.as_ref().map(|bi| bi.get_val())
    }

    fn valid(&self) -> bool {
        self.err.is_none()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.num_entries))
    }

    fn count(&self) -> usize
    where
        Self: Sized,
    {
        self.num_entries
    }
}

pub struct ConcatTableIterator {
    idx: isize,
    cur: Option<usize>,
    iters: Vec<UniTableIterator<RefCounter<RawTable>>>,
    tables: Vec<Table>,
    num_entries: usize,
    opt: Flag,
}

impl ConcatTableIterator {
    pub fn new(tables: Vec<Table>, opt: Flag) -> Self {
        let mut num_entries = 0;
        let idx = if (opt & Flag::REVERSED).bits() == 0 {
            0
        } else {
            tables.len() - 1
        } as isize;

        let iters = tables
            .iter()
            .map(|t| {
                num_entries += t.num_entries();
                t.iter(opt)
            })
            .collect();

        Self {
            idx,
            cur: None,
            iters,
            tables,
            num_entries,
            opt,
        }
    }

    fn set_idx(&mut self, idx: isize) {
        if idx < 0 || idx >= self.iters.len() as isize {
            self.cur = None;
            return;
        }
        self.idx = idx;
        let idx = idx as usize;
        self.cur = Some(idx);
    }

    fn rewind_in(&mut self) {
        if self.iters.is_empty() {
            return;
        }

        if (self.opt & Flag::REVERSED).bits() == 0 {
            self.set_idx(0);
        } else {
            self.set_idx(self.iters.len() as isize - 1);
        }
    }

    fn get_seek_position(&self, key: &[u8]) -> isize {
        let len = self.iters.len() as isize;
        if (self.opt & Flag::REVERSED).bits() == 0 {
            binary_search(len, |i| {
                match self.tables[i as usize]
                    .biggest()
                    .as_key_ref()
                    .compare_key(key)
                {
                    core::cmp::Ordering::Less => false,
                    core::cmp::Ordering::Equal | core::cmp::Ordering::Greater => true,
                }
            })
        } else {
            len - 1
                - binary_search(len, |i| {
                    match self.tables[(len - 1 - i) as usize]
                        .smallest()
                        .as_key_ref()
                        .compare_key(key)
                    {
                        core::cmp::Ordering::Less | core::cmp::Ordering::Equal => true,
                        core::cmp::Ordering::Greater => false,
                    }
                })
        }
    }
}

impl BableIterator for ConcatTableIterator {
    fn next(&mut self) {
        let idx = self.idx as usize;
        let cur_iter = &mut self.iters[idx];
        cur_iter.next();
        if cur_iter.valid() {
            return;
        }

        // In case there are empty tables.
        loop {
            if (self.opt & Flag::REVERSED).bits() == 0 {
                self.set_idx(self.idx + 1);
            } else {
                self.set_idx(self.idx - 1);
            }
            match self.cur {
                // End of list. Valid will become false.
                None => return,
                Some(cur) => {
                    let cur = &mut self.iters[cur];
                    cur.rewind();
                    if cur.valid() {
                        break;
                    }
                }
            }
        }
    }

    fn rewind(&mut self) {
        self.rewind_in();
        match self.cur {
            None => {}
            Some(cur) => self.iters[cur].rewind(),
        }
    }

    // seek brings us to element >= key if reversed is false. Otherwise, <= key.
    fn seek(&mut self, key: impl KeyExt) {
        let idx = self.get_seek_position(key.as_key_ref().as_bytes());
        let len = self.iters.len() as isize;
        if idx >= len || idx < 0 {
            self.set_idx(-1);
            return;
        }

        self.set_idx(idx);
        match self.cur {
            None => {}
            Some(cur) => self.iters[cur].seek(key.as_key_ref()),
        }
    }

    fn entry(&self) -> Option<(KeyRef, ValueRef)> {
        match self.cur {
            None => None,
            Some(cur) => self.iters[cur].entry(),
        }
    }

    fn key(&self) -> Option<KeyRef> {
        match self.cur {
            None => None,
            Some(cur) => self.iters[cur].key(),
        }
    }

    fn val(&self) -> Option<ValueRef> {
        match self.cur {
            None => None,
            Some(cur) => self.iters[cur].val(),
        }
    }

    fn valid(&self) -> bool {
        match self.cur {
            None => false,
            Some(cur) => self.iters[cur].valid(),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.num_entries))
    }

    fn count(&self) -> usize
    where
        Self: Sized,
    {
        self.num_entries
    }
}

pub struct MergeTableIterator {
    left: Node,
    right: Node,

    /// is left less than right
    lltr: bool,

    cur_key: BytesMut,
    reverse: bool,
    num_entries: usize,
}

impl MergeTableIterator {
    pub fn new(mut iters: Vec<Box<TableIterator>>, reverse: bool) -> Option<Box<TableIterator>> {
        let num_entries: usize = iters.iter().map(|it| it.count()).sum();
        match iters.len() {
            0 => None,
            1 => iters.pop(),
            2 => {
                let right = iters.pop().unwrap();
                let left = iters.pop().unwrap();
                Some(Box::new(TableIterator::from(MergeTableIterator {
                    reverse,
                    left: Node::new(left),
                    right: Node::new(right),
                    lltr: true,
                    cur_key: BytesMut::new(),
                    num_entries,
                })))
            }
            _ => {
                let mid = iters.len() / 2;
                let right = iters.split_off(mid);
                Some(Box::new(TableIterator::from(MergeTableIterator {
                    left: Node::new(MergeTableIterator::new(iters, reverse).unwrap()),
                    right: Node::new(MergeTableIterator::new(right, reverse).unwrap()),
                    lltr: true,
                    cur_key: Default::default(),
                    reverse,
                    num_entries,
                })))
            }
        }
    }

    fn fix(&mut self) {
        if !self.bigger().valid {
            return;
        }

        if !self.smaller().valid {
            self.swap_small();
            return;
        }

        match self.smaller().key.cmp(&self.bigger().key) {
            // Small is less than bigger().
            core::cmp::Ordering::Less => {
                if self.reverse {
                    self.swap_small()
                } else {
                    // we don't need to do anything. Small already points to the smallest.
                }
            }
            // Both the keys are equal.
            core::cmp::Ordering::Equal => {
                // In case of same keys, move the right iterator ahead.
                self.right.next();
                if !self.lltr {
                    self.swap_small();
                }
            }
            // bigger() is less than small.
            core::cmp::Ordering::Greater => {
                if self.reverse {
                    // Do nothing since we're iterating in reverse. Small currently points to
                    // the bigger key and that's okay in reverse iteration.
                } else {
                    self.swap_small();
                }
            }
        }
    }

    #[inline]
    fn smaller(&self) -> &Node {
        if self.lltr {
            &self.left
        } else {
            &self.right
        }
    }

    #[inline]
    fn smaller_mut(&mut self) -> &mut Node {
        if self.lltr {
            &mut self.left
        } else {
            &mut self.right
        }
    }

    #[inline]
    fn bigger(&self) -> &Node {
        if !self.lltr {
            &self.left
        } else {
            &self.right
        }
    }

    #[inline]
    fn swap_small(&mut self) {
        self.lltr = !self.lltr;
    }

    #[inline]
    fn set_current(&mut self) {
        self.cur_key.clear();
        if self.lltr {
            self.cur_key.extend_from_slice(&self.left.key);
        } else {
            self.cur_key.extend_from_slice(&self.right.key);
        }
    }
}

impl BableIterator for MergeTableIterator {
    #[inline]
    fn next(&mut self) {
        while self.valid() {
            let small = self.smaller();
            if small.key != self.cur_key {
                break;
            }
            self.smaller_mut().next();
            self.fix();
        }
        self.set_current();
    }

    #[inline]
    fn rewind(&mut self) {
        self.left.rewind();
        self.right.rewind();
        self.fix();
        self.set_current();
    }

    #[inline]
    fn seek(&mut self, key: impl KeyExt) {
        let key_ref = key.as_key_ref();
        self.left.seek(key_ref);
        self.right.seek(key_ref);
        self.fix();
        self.set_current();
    }

    #[inline]
    fn entry(&self) -> Option<(KeyRef, ValueRef)> {
        self.smaller().iter.entry()
    }

    #[inline]
    fn key(&self) -> Option<KeyRef> {
        self.smaller().iter.key()
    }

    #[inline]
    fn val(&self) -> Option<ValueRef> {
        self.smaller().iter.val()
    }

    #[inline]
    fn valid(&self) -> bool {
        self.smaller().valid
    }

    #[inline]
    fn count(&self) -> usize
    where
        Self: Sized,
    {
        self.num_entries
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.num_entries))
    }
}

struct Node {
    valid: bool,
    key: BytesMut,
    iter: Box<TableIterator>,
}

impl Node {
    #[inline(always)]
    fn new(iter: Box<TableIterator>) -> Self {
        Self {
            valid: false,
            key: BytesMut::new(),
            iter,
        }
    }

    #[inline(always)]
    fn set_key(&mut self) {
        self.valid = self.iter.valid();
        if self.valid {
            self.key.clear();

            match self.iter.key() {
                None => {}
                Some(k) => self.key.extend_from_slice(k.as_bytes()),
            }
        }
    }

    #[inline(always)]
    fn next(&mut self) {
        self.iter.next();
        self.set_key();
    }

    #[inline(always)]
    fn rewind(&mut self) {
        self.iter.rewind();
        self.set_key();
    }

    #[inline(always)]
    fn seek(&mut self, key: impl KeyExt) {
        self.iter.seek(key);
        self.set_key();
    }
}
