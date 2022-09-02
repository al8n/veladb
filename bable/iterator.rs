mod iterator {
    use super::*;
    use crate::{binary_search, HEADER_SIZE, Header};
    use vpb::kvstructs::{
        KeyExt, KeyRef, ValueRef, ValueExt,
        bytes::{Buf, Bytes, BytesMut},
        iterator::{SeekFrom, Iterator as BableIterator},
    };
    use core::iter::FusedIterator;
    const REVERSED: usize = 2;
    const NO_CACHE: usize = 4;
    pub enum IterError {
        EOF,
        Other(String),
    }
    #[automatically_derived]
    impl ::core::clone::Clone for IterError {
        #[inline]
        fn clone(&self) -> IterError {
            match self {
                IterError::EOF => IterError::EOF,
                IterError::Other(__self_0) => {
                    IterError::Other(::core::clone::Clone::clone(__self_0))
                }
            }
        }
    }
    impl ::core::marker::StructuralEq for IterError {}
    #[automatically_derived]
    impl ::core::cmp::Eq for IterError {
        #[inline]
        #[doc(hidden)]
        #[no_coverage]
        fn assert_receiver_is_total_eq(&self) -> () {
            let _: ::core::cmp::AssertParamIsEq<String>;
        }
    }
    impl ::core::marker::StructuralPartialEq for IterError {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for IterError {
        #[inline]
        fn eq(&self, other: &IterError) -> bool {
            let __self_tag = ::core::intrinsics::discriminant_value(self);
            let __arg1_tag = ::core::intrinsics::discriminant_value(other);
            __self_tag == __arg1_tag
                && match (self, other) {
                    (IterError::Other(__self_0), IterError::Other(__arg1_0)) => {
                        *__self_0 == *__arg1_0
                    }
                    _ => true,
                }
        }
        #[inline]
        fn ne(&self, other: &IterError) -> bool {
            let __self_tag = ::core::intrinsics::discriminant_value(self);
            let __arg1_tag = ::core::intrinsics::discriminant_value(other);
            __self_tag != __arg1_tag
                || match (self, other) {
                    (IterError::Other(__self_0), IterError::Other(__arg1_0)) => {
                        *__self_0 != *__arg1_0
                    }
                    _ => false,
                }
        }
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for IterError {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match self {
                IterError::EOF => ::core::fmt::Formatter::write_str(f, "EOF"),
                IterError::Other(__self_0) => {
                    ::core::fmt::Formatter::debug_tuple_field1_finish(f, "Other", &__self_0)
                }
            }
        }
    }
    impl core::fmt::Display for IterError {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match self {
                IterError::EOF => f.write_fmt(::core::fmt::Arguments::new_v1(&["EOF"], &[])),
                IterError::Other(s) => f.write_fmt(::core::fmt::Arguments::new_v1(
                    &[""],
                    &[::core::fmt::ArgumentV1::new_display(&s)],
                )),
            }
        }
    }
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
            (
                KeyRef::new(self.key.as_ref()),
                ValueRef::decode_value_ref(self.val.as_ref()),
            )
        }
        pub fn set_block(&mut self, block: RefCounter<Block>) {
            self.err = None;
            self.idx = 0;
            self.base_key.clear();
            self.prev_overlap = 0;
            self.key.clear();
            self.val.clear();
            self.data = block.data();
            self.block = block;
        }
        #[inline(always)]
        pub fn valid(&self) -> bool {
            self.err.is_none()
        }
        #[inline(always)]
        pub fn error(&self) -> Option<&IterError> {
            self.err.as_ref()
        }
        /// seek brings us to the first block element that is >= input key.
        pub(crate) fn seek(&mut self, key: &[u8], whence: SeekFrom) {
            self.err = None;
            let start_index = match whence {
                SeekFrom::Origin => 0,
                SeekFrom::Current => self.idx,
            };
            let found_entry_index = binary_search(self.block.entry_offsets.len() as isize, |idx| {
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
            if self.base_key.len() == 0 {
                let h = Header::decode(self.data.as_ref());
                self.base_key = self
                    .data
                    .slice(HEADER_SIZE..HEADER_SIZE + h.diff() as usize);
            }
            let end_offset = if self.idx + 1 == block_entry_offsets_len {
                self.data.len()
            } else {
                self.block.entry_offsets[idx + 1] as usize
            };
            let entry_data = self.data.slice(start_offset..end_offset);
            let h = Header::decode(entry_data.as_ref());
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
            let _guard = ::scopeguard::guard_on_unwind((), |()| {
                let mut debug_buf = String::new();
                debug_buf.push_str("==== Recovered ====\n");
                debug_buf.push_str(
                    {
                        let res = ::alloc::fmt::format(::core::fmt::Arguments::new_v1(
                            &[
                                "Table ID: ",
                                "\nBlock Idx: ",
                                "\nEntry Idx: ",
                                "\nData len: ",
                                "\nStartOffset: ",
                                "\nEndOffset: ",
                                "\nEntryOffsets len: ",
                                "\nEntryOffsets: ",
                                "\n",
                            ],
                            &[
                                ::core::fmt::ArgumentV1::new_display(&self.table_id),
                                ::core::fmt::ArgumentV1::new_display(&self.block_id),
                                ::core::fmt::ArgumentV1::new_display(&self.idx),
                                ::core::fmt::ArgumentV1::new_display(&self.data.len()),
                                ::core::fmt::ArgumentV1::new_display(&start_offset),
                                ::core::fmt::ArgumentV1::new_display(&end_offset),
                                ::core::fmt::ArgumentV1::new_display(
                                    &self.block.entry_offsets.len(),
                                ),
                                ::core::fmt::ArgumentV1::new_debug(&self.block.entry_offsets),
                            ],
                        ));
                        res
                    }
                    .as_str(),
                );
                {
                    use ::tracing::__macro_support::Callsite as _;
                    static CALLSITE: ::tracing::callsite::DefaultCallsite = {
                        static META: ::tracing::Metadata<'static> = {
                            ::tracing_core::metadata::Metadata::new(
                                "event bable/src/table/iterator.rs:213",
                                "table_iterator",
                                ::tracing::Level::ERROR,
                                Some("bable/src/table/iterator.rs"),
                                Some(213u32),
                                Some("bable::table::iterator"),
                                ::tracing_core::field::FieldSet::new(
                                    &["message"],
                                    ::tracing_core::callsite::Identifier(&CALLSITE),
                                ),
                                ::tracing::metadata::Kind::EVENT,
                            )
                        };
                        ::tracing::callsite::DefaultCallsite::new(&META)
                    };
                    let enabled = ::tracing::Level::ERROR
                        <= ::tracing::level_filters::STATIC_MAX_LEVEL
                        && ::tracing::Level::ERROR
                            <= ::tracing::level_filters::LevelFilter::current()
                        && {
                            let interest = CALLSITE.interest();
                            !interest.is_never()
                                && ::tracing::__macro_support::__is_enabled(
                                    CALLSITE.metadata(),
                                    interest,
                                )
                        };
                    if enabled {
                        (|value_set: ::tracing::field::ValueSet| {
                            let meta = CALLSITE.metadata();
                            ::tracing::Event::dispatch(meta, &value_set);
                        })({
                            #[allow(unused_imports)]
                            use ::tracing::field::{debug, display, Value};
                            let mut iter = CALLSITE.metadata().fields().iter();
                            CALLSITE.metadata().fields().value_set(&[(
                                &iter.next().expect("FieldSet corrupted (this is a bug)"),
                                Some(&::core::fmt::Arguments::new_v1(
                                    &[""],
                                    &[::core::fmt::ArgumentV1::new_display(&debug_buf)],
                                ) as &Value),
                            )])
                        });
                    } else {
                    }
                };
            });
        }
    }
    impl<'a> core::iter::Iterator for BlockIter {
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
        ConcatTableIterator(ConcatTableIterator),
        MergeTableIterator(MergeTableIterator),
        UniTableIterator(UniTableIterator<RefCounter<RawTable>>),
    }
    pub struct UniTableIterator<I: AsRef<RawTable>> {
        table: I,
        bpos: isize,
        bi: Option<BlockIter>,
        err: Option<IterError>,
        num_entries: usize,
        opt: usize,
    }
    impl<I: AsRef<RawTable>> UniTableIterator<I> {
        pub(crate) fn new(t: I, opt: usize) -> Self {
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
        #[inline(always)]
        pub(crate) fn t(&self) -> &RawTable {
            self.table.as_ref()
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
            (self.opt & NO_CACHE) == 0
        }
        /// seek_from_key brings us to a key that is >= input key.
        fn seek_from_key(&mut self, key: &[u8], from: SeekFrom) {
            let idx = self.get_seek_position(key, from);
            if idx == 0 {
                self.seek_helper(0, key);
                return;
            }
            self.seek_helper(idx - 1, key);
            if let Some(err) = self.err.as_ref() {
                match err {
                    IterError::EOF => {
                        if idx == self.table.as_ref().offsets_length() as isize {
                            return;
                        } else {
                            self.seek_helper(idx, key);
                        }
                    }
                    IterError::Other(_) => {}
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
            if self.bi.is_some() {
                let mut bi = self.bi.as_mut().unwrap();
                if bi.data.is_empty() {
                    let block = self.table.as_ref().block(self.bpos, use_cache);
                    match block {
                        Ok(block) => {
                            bi.seek_to_block(block, table_id, self.bpos, false);
                            self.err = bi.err.clone();
                            return;
                        }
                        Err(e) => {
                            self.err = Some(IterError::Other(e.to_string()));
                            return;
                        }
                    }
                } else {
                    bi.next();
                    if !bi.valid() {
                        self.bpos += 1;
                        bi.data.clear();
                        self.next_in();
                        return;
                    }
                }
            } else {
                self.err = Some(IterError::EOF);
                return;
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
            if self.bi.is_some() {
                let mut bi = self.bi.as_mut().unwrap();
                if bi.data.len() == 0 {
                    let block = self.table.as_ref().block(self.bpos, use_cache);
                    match block {
                        Ok(block) => {
                            bi.seek_to_block(block, table_id, self.bpos, true);
                            self.err = bi.err.clone();
                            return;
                        }
                        Err(e) => {
                            self.err = Some(IterError::Other(e.to_string()));
                            return;
                        }
                    }
                } else {
                    bi.prev();
                    if !bi.valid() {
                        self.bpos = self.bpos.overflowing_sub(1).0;
                        bi.data.clear();
                        self.prev();
                        return;
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
    impl<'a, I: AsRef<RawTable>> BableIterator<KeyRef<'a>, ValueRef<'a>> for UniTableIterator<I> {
        fn next(&mut self) {
            if self.bpos == -1 {
                self.rewind();
            } else if self.opt & REVERSED == 0 {
                self.next_in();
            } else {
                self.prev();
            }
        }
        fn rewind(&mut self) {
            if self.opt & REVERSED == 0 {
                self.seek_to_first()
            } else {
                self.seek_to_last()
            }
        }
        fn seek<K: KeyExt>(&mut self, key: K) {
            if self.opt & REVERSED == 0 {
                self.seek_from_key(key.as_bytes(), SeekFrom::Origin)
            } else {
                self.seek_to_key(key)
            }
        }
        fn entry(&self) -> Option<(KeyRef<'a>, ValueRef<'a>)> {
            match self.bi {
                None => None,
                Some(ref bi) => Some(bi.entry()),
            }
        }
        fn key(&self) -> Option<KeyRef<'a>> {
            match self.bi {
                None => None,
                Some(ref bi) => Some(bi.get_key()),
            }
        }
        fn val(&self) -> Option<ValueRef<'a>> {
            match self.bi {
                None => None,
                Some(ref bi) => Some(bi.get_val()),
            }
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
        opt: usize,
    }
    impl ConcatTableIterator {
        pub fn new(tables: Vec<Table>, opt: usize) -> Self {
            let mut num_entries = 0;
            let idx = if opt & REVERSED == 0 {
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
            if self.iters.len() == 0 {
                return;
            }
            if self.opt & REVERSED == 0 {
                self.set_idx(0);
            } else {
                self.set_idx(self.iters.len() as isize - 1);
            }
        }
        fn get_seek_position(&self, key: &[u8]) -> isize {
            let mut idx;
            let len = self.iters.len() as isize;
            if self.opt & REVERSED == 0 {
                idx = binary_search(len, |i| {
                    match self.tables[i as usize]
                        .biggest()
                        .as_key_ref()
                        .compare_key(key)
                    {
                        core::cmp::Ordering::Less => false,
                        core::cmp::Ordering::Equal | core::cmp::Ordering::Greater => true,
                    }
                });
            } else {
                idx = len
                    - 1
                    - binary_search(len, |i| {
                        match self.tables[(len - 1 - i) as usize]
                            .smallest()
                            .as_key_ref()
                            .compare_key(key)
                        {
                            core::cmp::Ordering::Less | core::cmp::Ordering::Equal => true,
                            core::cmp::Ordering::Greater => false,
                        }
                    });
            }
            idx
        }
    }
    impl<'a> BableIterator<KeyRef<'a>, ValueRef<'a>> for ConcatTableIterator {
        fn next(&mut self) {
            let idx = self.idx as usize;
            let cur_iter = &mut self.iters[idx];
            vpb::kvstructs::iterator::Iterator::next(cur_iter);
            if cur_iter.valid() {
                return;
            }
            loop {
                if self.opt & REVERSED == 0 {
                    self.set_idx(self.idx + 1);
                } else {
                    self.set_idx(self.idx - 1);
                }
                match self.cur {
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
        fn seek<K: KeyExt>(&mut self, key: K) {
            let mut idx = self.get_seek_position(key.as_key_ref().as_bytes());
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
        fn entry(&self) -> Option<(KeyRef<'a>, ValueRef<'a>)> {
            match self.cur {
                None => None,
                Some(cur) => self.iters[cur].entry(),
            }
        }
        fn key(&self) -> Option<KeyRef<'a>> {
            match self.cur {
                None => None,
                Some(cur) => self.iters[cur].key(),
            }
        }
        fn val(&self) -> Option<ValueRef<'a>> {
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
        pub fn new(
            mut iters: Vec<Box<TableIterator>>,
            reverse: bool,
        ) -> Option<Box<TableIterator>> {
            let num_entries: usize = iters.iter().map(|it| it.count()).sum();
            match iters.len() {
                0 => None,
                1 => iters.pop(),
                2 => {
                    let right = iters.pop().unwrap();
                    let left = iters.pop().unwrap();
                    Some(Box::new(<TableIterator>::from(MergeTableIterator {
                        reverse,
                        left: <Node>::new(left),
                        right: <Node>::new(right),
                        lltr: true,
                        cur_key: BytesMut::new(),
                        num_entries,
                    })))
                }
                _ => {
                    let mid = iters.len() / 2;
                    let right = iters.split_off(mid);
                    Some(Box::new(<TableIterator>::from(MergeTableIterator {
                        left: <Node>::new(MergeTableIterator::new(iters, reverse).unwrap()),
                        right: <Node>::new(MergeTableIterator::new(right, reverse).unwrap()),
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
                core::cmp::Ordering::Less => {
                    if self.reverse {
                        self.swap_small()
                    } else {
                    }
                }
                core::cmp::Ordering::Equal => {
                    self.right.next();
                    if !self.lltr {
                        self.swap_small();
                    }
                }
                core::cmp::Ordering::Greater => {
                    if self.reverse {
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
    impl<'a> BableIterator<KeyRef<'a>, ValueRef<'a>> for MergeTableIterator {
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
        fn seek<K: KeyExt>(&mut self, key: K) {
            let key_ref = key.as_key_ref();
            self.left.seek(key_ref);
            self.right.seek(key_ref);
            self.fix();
            self.set_current();
        }
        #[inline]
        fn entry(&self) -> Option<(KeyRef<'a>, ValueRef<'a>)> {
            BableIterator::entry(&self.smaller().iter)
        }
        #[inline]
        fn key(&self) -> Option<KeyRef<'a>> {
            self.smaller().iter.key()
        }
        #[inline]
        fn val(&self) -> Option<ValueRef<'a>> {
            self.smaller().iter.val()
        }
        #[inline]
        fn valid(&self) -> bool {
            self.smaller().valid
        }
    }
    struct Node {
        valid: bool,
        key: BytesMut,
        iter: Box<TableIterator>,
    }
    impl Node {
        fn new(iter: Box<TableIterator>) -> Self {
            Self {
                valid: false,
                key: BytesMut::new(),
                iter,
            }
        }
        fn set_iterator(&mut self, iter: TableIterator) {
            self.iter = Box::new(iter);
        }
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
        fn next(&mut self) {
            self.iter.next();
            self.set_key();
        }
        fn rewind(&mut self) {
            self.iter.rewind();
            self.set_key();
        }
        fn seek<K: KeyExt>(&mut self, key: K) {
            self.iter.seek(key);
            self.set_key();
        }
    }
}
