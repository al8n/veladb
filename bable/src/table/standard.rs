use std::path::{Path, PathBuf};

use fmmap::{MetaDataExt, MmapFileExt, MmapFileMut, MmapFileMutExt};
use vpb::{
    checksum::Checksumer,
    compression::Compressor,
    encrypt::Encryptor,
    kvstructs::{
        bytes::{BufMut, Bytes, BytesMut},
        Key, KeyExt,
    },
    BlockOffset, Checksum, Compression, Encryption, Marshaller, TableIndex,
};

use super::{
    iterator::{BableIterator, Flag, UniTableIterator},
    CheapIndex, ChecksumVerificationMode, Options, TableBuilder, FILE_SUFFIX,
};
use crate::{bloom::MayContain, error::*, Block, RefCounter};

pub trait TableData {
    fn size(&self) -> usize;

    fn write(self, writer: &mut MmapFileMut) -> Result<usize>;
}

impl super::Table {
    pub fn create_table<P: AsRef<Path>>(path: P, builder: impl TableBuilder) -> Result<Self> {
        let opts = builder.options();
        let bd = builder.build()?;
        if let Some(bd) = bd {
            let mut mmap = fmmap::Options::new()
                .max_size(bd.size() as u64)
                .create_mmap_file_mut(&path)?;
            mmap.set_remove_on_drop(true);

            // TODO: check written bytes
            let _written = bd.write(&mut mmap)?;
            mmap.flush()
                .map_err(|e| {
                    #[cfg(feature = "tracing")]
                    {
                        tracing::error!(target: "table", err = %e, info = format!("while calling msync on {:?}", path.as_ref().to_string_lossy()));
                    }
                    Error::MmapError(e)
                })
                .and_then(|_| Self::open_table(mmap, opts))
        } else {
            MmapFileMut::create(path)
                .map_err(Error::MmapError)
                .and_then(|mut mmap| {
                    mmap.set_remove_on_drop(true);
                    Self::open_table(mmap, opts)
                })
        }
    }

    pub fn create_table_from_buffer<P: AsRef<Path>>(
        path: P,
        buf: Bytes,
        opts: RefCounter<Options>,
    ) -> Result<Self> {
        let mut mmap = fmmap::Options::new()
            .max_size(buf.len() as u64)
            .create_mmap_file_mut(&path)?;
        mmap.set_remove_on_drop(true);
        // We cannot use the buf directly here because it is not mmapped.
        mmap.write(buf.as_ref(), 0);
        mmap.flush()?;
        Self::open_table(mmap, opts)
    }

    /// open_in_memory_table is similar to OpenTable but it opens a new table from the provided data.
    /// open_in_memory_table is used for L0 tables.
    pub fn open_in_memory_table(data: Vec<u8>, id: u64, opts: RefCounter<Options>) -> Result<Self> {
        let tbl_size = data.len();
        let mut this = RawTable {
            mmap: MmapFileMut::memory_from_vec("memory.sst", data),
            table_size: tbl_size,
            index: RefCounter::new(TableIndex::new()),
            cheap: CheapIndex {
                max_version: 0,
                key_count: 0,
                uncompressed_size: 0,
                on_disk_size: 0,
                bloom_filter_length: 0,
                offsets_length: 0,
                num_entries: 0,
            },
            smallest: Default::default(),
            biggest: Default::default(),
            id, // It is important that each table gets a unique ID.
            checksum: Default::default(),
            created_at: std::time::SystemTime::now(),
            index_start: 0,
            index_len: 0,
            has_bloom_filter: false,
            in_memory: true,
            opts,
        };
        this.init_biggest_and_smallest();
        Ok(this.into())
    }

    /// open_table assumes file has only one table and opens it. Takes ownership of fd upon function
    /// entry. The fd has to writeable because we call Truncate on it before
    /// deleting. Checksum for all blocks of table is verified based on value of chkMode.
    pub fn open_table(mf: MmapFileMut, opts: RefCounter<Options>) -> Result<Self> {
        // BlockSize is used to compute the approximate size of the decompressed
        // block. It should not be zero if the table is compressed.
        if opts.block_size() == 0 && !opts.compression().is_none() {
            return Err(Error::EmptyBlock);
        }

        let (id, ok) = Self::parse_file_id(MmapFileExt::path(&mf));
        if !ok {
            return Err(Error::InvalidFile(format!("{:?}", MmapFileExt::path(&mf))));
        }

        let meta = MmapFileExt::metadata(&mf)?;
        let mut this = RawTable {
            mmap: mf,
            table_size: meta.size() as usize,
            index: RefCounter::new(Default::default()),
            cheap: CheapIndex {
                max_version: 0,
                key_count: 0,
                uncompressed_size: 0,
                on_disk_size: 0,
                bloom_filter_length: 0,
                offsets_length: 0,
                num_entries: 0,
            },
            smallest: Default::default(),
            biggest: Default::default(),
            id,
            checksum: Default::default(),
            created_at: meta.modified()?,
            index_start: 0,
            index_len: 0,
            has_bloom_filter: false,
            opts,
            in_memory: false,
        };
        this.init_biggest_and_smallest();

        if matches!(
            this.opts.checksum_verification_mode(),
            ChecksumVerificationMode::OnTableRead | ChecksumVerificationMode::OnTableAndBlockRead
        ) {
            return this.verify_checksum().map(|_| this.into()).map_err(|e| {
                #[cfg(feature = "tracing")]
                {
                    tracing::error!(target: "table", err = %e, info = "failed to verify checksum");
                }
                e
            });
        }
        Ok(this.into())
    }

    /// Returns the creation time for the table.
    #[inline]
    pub fn created_at(&self) -> std::time::SystemTime {
        self.created_at
    }

    /// Does the inverse of ParseFileID
    #[inline]
    pub fn id_to_filename(id: u64) -> PathBuf {
        let mut pb = PathBuf::from(format!("{:06}", id));
        pb.set_extension(FILE_SUFFIX);
        pb
    }

    /// parse_file_id reads the file id out of a filename.
    #[inline]
    pub fn parse_file_id<P: AsRef<Path>>(path: P) -> (u64, bool) {
        if let Some(ext) = path.as_ref().extension() {
            if !ext.eq(FILE_SUFFIX) {
                return (0, false);
            }

            if let Some(name) = path.as_ref().file_stem() {
                name.to_str()
                    .map(|sid| {
                        sid.parse::<u64>()
                            .map(|id| (id, true))
                            .unwrap_or((0, false))
                    })
                    .unwrap_or((0, false))
            } else {
                (0, false)
            }
        } else {
            (0, false)
        }
    }

    /// Should be named TableFilepath -- it combines the dir with the ID to make a table
    /// filepath.
    #[inline]
    pub fn new_filename(id: u64, mut dir: PathBuf) -> PathBuf {
        dir.push(format!("{:06}", id));
        dir.set_extension(FILE_SUFFIX);
        dir
    }
}

pub struct RawTable {
    mmap: MmapFileMut,
    /// Initialized in OpenTable, using fd.Stat().
    table_size: usize,

    // Use fetch_index to access.
    index: RefCounter<TableIndex>,
    cheap: CheapIndex,

    // The following are initialized once and const.
    /// Smallest keys (with timestamps).
    smallest: Key,
    /// Largest keys (with timestamps).
    biggest: Key,

    // file id, part of filename
    id: u64,

    checksum: Bytes,

    created_at: std::time::SystemTime,
    index_start: usize,
    index_len: usize,
    has_bloom_filter: bool,
    in_memory: bool,
    opts: RefCounter<Options>,
}

impl AsRef<RawTable> for RawTable {
    fn as_ref(&self) -> &RawTable {
        self
    }
}

impl RawTable {
    #[inline]
    pub(super) fn biggest(&self) -> &Key {
        &self.biggest
    }

    #[inline]
    pub(super) fn smallest(&self) -> &Key {
        &self.smallest
    }

    #[inline]
    pub(super) fn id(&self) -> u64 {
        self.id
    }

    #[inline]
    pub(super) fn path(&self) -> &std::path::Path {
        self.mmap.path()
    }

    #[inline]
    pub(super) fn max_version(&self) -> u64 {
        self.cheap.max_version
    }

    #[inline]
    pub(super) fn bloom_filter_size(&self) -> usize {
        self.cheap.bloom_filter_length
    }

    #[inline]
    pub(super) fn uncompressed_size(&self) -> u32 {
        self.cheap.uncompressed_size
    }

    #[inline]
    pub(super) fn key_count(&self) -> u32 {
        self.cheap.key_count
    }

    #[inline]
    pub(super) fn num_entries(&self) -> usize {
        self.cheap.num_entries
    }

    #[inline]
    pub(super) fn on_disk_size(&self) -> u32 {
        self.cheap.on_disk_size
    }

    #[inline]
    pub(super) fn secret(&self) -> &[u8] {
        self.opts.encryption().secret()
    }

    #[inline]
    pub(super) fn compression(&self) -> Compression {
        self.opts.compression()
    }

    #[inline]
    pub(super) fn checksum(&self) -> &[u8] {
        &self.checksum
    }

    #[inline]
    pub(super) fn checksum_bytes(&self) -> &Bytes {
        &self.checksum
    }

    #[inline]
    pub(super) const fn in_memory(&self) -> bool {
        self.in_memory
    }

    #[inline]
    pub(super) const fn table_size(&self) -> usize {
        self.table_size
    }

    // TODO: optimize clone on RefCounter
    #[inline]
    pub(super) fn stale_data_size(&self) -> usize {
        self.fetch_index().stale_data_size as usize
    }

    /// Splits the table into at least n ranges based on the block offsets.
    pub(super) fn key_splits(&self, idx: usize, prefix: &[u8]) -> Vec<Key> {
        let mut res = Vec::new();
        if idx == 0 {
            return res;
        }

        let offsets_len = self.offsets_length();
        let mut idx = 0;
        while idx < offsets_len {
            if idx >= offsets_len {
                idx = offsets_len - 1;
            }
            let bo = &self.fetch_index().offsets[idx];
            if bo.key.has_prefix(prefix) {
                res.push(bo.key.clone().into());
            }
            idx += offsets_len;
        }
        res
    }

    #[inline]
    pub(super) fn iter(&self, opt: Flag) -> UniTableIterator<&RawTable> {
        UniTableIterator::new(self, opt)
    }

    #[inline]
    pub(super) fn offsets_length(&self) -> usize {
        self.cheap.offsets_length
    }

    #[inline(always)]
    fn read(&self, offset: usize, sz: usize) -> Result<&[u8]> {
        self.mmap.bytes(offset, sz).map_err(|e| e.into())
    }

    #[inline(always)]
    fn read_no_fail(&self, offset: usize, sz: usize) -> &[u8] {
        self.mmap.bytes(offset, sz).unwrap()
    }

    /// Returns true if and only if the table does not have the key hash.
    /// It does a bloom filter lookup.
    #[inline]
    pub(super) fn contains_hash(&self, hash: u32) -> bool {
        if !self.has_bloom_filter {
            return false;
        }

        #[cfg(feature = "metrics")]
        {
            crate::metrics::BLOOM_HITS.fetch_add(crate::metrics::DOES_NOT_HAVE_ALL, 1)
        }
        let index = self.fetch_index();
        let may_contain = index.bloom_filter.may_contain(hash);
        #[cfg(feature = "metrics")]
        {
            if !may_contain {
                crate::metrics::BLOOM_HITS.fetch_add(crate::metrics::DOES_NOT_HAVE_HIT, 1)
            }
        }

        !may_contain
    }

    /// Returns true if all the keys in the table are prefixed by the given prefix.
    #[inline]
    pub(super) fn covered_by_prefix(&self, prefix: &[u8]) -> bool {
        self.biggest.parse_key().has_prefix(prefix) && self.smallest.parse_key().has_prefix(prefix)
    }

    /// Verifies checksum for all blocks of table. This function is called by
    /// OpenTable() function. This function is also called inside levelsController.VerifyChecksum().
    #[inline]
    pub(super) fn verify_checksum(&self) -> Result<()> {
        let index = self.fetch_index();
        for i in 0..(index.offsets.len() as isize) {
            let blk = self.block(i, true).map_err(|e| {
                #[cfg(feature = "tracing")]
                {
                    tracing::error!(target: "table", info = "checksum verification failed", err = %e);
                }
                Error::BlockChecksumMismatch { table: self.path().to_string_lossy().to_string(), block: i as usize}
            })?;
            // OnBlockRead or OnTableAndBlockRead, we don't need to call verify checksum
            // on block, verification would be done while reading block itself.
            if !matches!(
                self.opts.checksum_verification_mode(),
                ChecksumVerificationMode::OnBlockRead
                    | ChecksumVerificationMode::OnTableAndBlockRead
            ) {
                Block::verify_checksum(&blk, Options::checksum(&self.opts))
                    .map_err(|e| {
                        #[cfg(feature = "tracing")]
                        {
                            tracing::error!(target: "table", info = "checksum verification failed", err = %e);
                        }
                        Error::BlockOffsetChecksumMismatch { table: self.path().to_string_lossy().to_string(), block: i as usize, offset: blk.offset }
                })?;
            }
        }
        Ok(())
    }

    #[inline(always)]
    fn block_cache_key(&self, idx: usize) -> [u8; core::mem::size_of::<u64>()] {
        let mut key = [0u8; core::mem::size_of::<u64>()];
        assert!(self.id < u32::MAX as u64);
        assert!(u32::MAX > idx as u32);

        key[..4].copy_from_slice(&(self.id as u32).to_be_bytes());
        key[4..].copy_from_slice(&(idx as u32).to_be_bytes());
        key
    }

    /// block function return a new block.
    pub(super) fn block(&self, idx: isize, use_cache: bool) -> Result<RefCounter<Block>> {
        let blk = self.block_inner(idx)?;
        if use_cache {
            self.insert_block_to_caches(idx, blk.clone());
        }
        Ok(blk)
    }

    #[inline(always)]
    fn insert_block_to_caches(&self, idx: isize, blk: RefCounter<Block>) {
        if let Some(cache) = self.opts.block_cache() {
            let key = self.block_cache_key(idx as usize);
            let blk_size = blk.size() as i64;
            let mut k = BytesMut::with_capacity(core::mem::size_of::<u64>());
            k.put_slice(&key);
            #[cfg(feature = "std")]
            cache.insert(k.freeze(), blk, blk_size);
            #[cfg(not(feature = "std"))]
            cache.insert(k.freeze(), blk.clone());
            // We have added an OnReject func in our cache, which gets called in case the block is not
            // admitted to the cache. So, every block would be accounted for.
        }
    }

    #[allow(clippy::unsound_collection_transmute)]
    #[inline(always)]
    fn block_inner(&self, idx: isize) -> Result<RefCounter<Block>> {
        assert!(idx >= 0, "idx={}", idx);
        if idx >= self.offsets_length() as isize {
            return Err(Error::BlockOutOfRange {
                num_offsets: self.offsets_length() as u64,
                index: idx as u64,
            });
        }

        let idx = idx as usize;
        if let Some(cache) = self.opts.block_cache() {
            let key = self.block_cache_key(idx);
            if let Some(blk) = cache.get(key.as_ref()) {
                return Ok(blk.value().clone());
            }
        }

        let index = self.fetch_index();
        let bo = &index.offsets[idx];
        let offset = bo.offset as usize;
        let bo_len = bo.len as usize;
        let data = self.read(offset, bo_len).map_err(|e| {
            #[cfg(feature = "tracing")]
            {
                tracing::error!(target: "table", err=%e, info = format!(
                    "failed to read from table: {:?} at offset: {}, len: {}",
                    self.path(),
                    offset,
                    bo_len
                ));
            }
            e
        })?;

        let encryption = self.opts.encryption();

        let data: Bytes = if encryption.is_some() {
            let v = self.decrypt(data)?;
            v.decompress_into_vec(self.opts.compression())
                .map(From::from)
                .map_err(Error::Compression)?
        } else {
            data.decompress_into_vec(self.opts.compression())
                .map(From::from)
                .map_err(Error::Compression)?
        };

        let blk_data_len = data.len();
        // Read meta data related to block.
        let mut read_pos = blk_data_len - 4; // First read checksum length
        let cks_len =
            u32::from_be_bytes((&data[read_pos..read_pos + 4]).try_into().unwrap()) as usize;

        // Checksum length greater than block size could happen if the table was compressed and
        // it was opened with an incorrect compression algorithm (or the data was corrupted).
        if cks_len > blk_data_len {
            return Err(Error::InvalidChecksumLength);
        }

        // Read checksum and store it
        read_pos -= cks_len;
        let checksum = data.slice(read_pos..read_pos + cks_len);
        // Move back and read numEntries in the block.
        read_pos -= 4;
        let num_entries =
            u32::from_be_bytes((&data[read_pos..read_pos + 4]).try_into().unwrap()) as usize;
        let entries_index_start = read_pos - (num_entries * 4);
        let entries_index_end = entries_index_start + (num_entries * 4);
        let mut entry_offsets = vec![0; num_entries];
        entry_offsets.copy_from_slice(bytes_to_u32_slice(
            &data[entries_index_start..entries_index_end],
        ));

        let blk = Block {
            offset,
            data: data.slice(..read_pos + 4),
            checksum,
            entries_index_start,
            entry_offsets,
        };

        // Drop checksum and checksum length.
        // The checksum is calculated for actual data + entry index + index length
        // Verify checksum on if checksum verification mode is OnRead on OnStartAndRead.
        if matches!(
            self.opts.checksum_verification_mode(),
            ChecksumVerificationMode::OnBlockRead | ChecksumVerificationMode::OnTableAndBlockRead
        ) {
            Block::verify_checksum(&blk, Options::checksum(&self.opts))?;
        }

        Ok(RefCounter::new(blk))
    }

    fn init_biggest_and_smallest(&mut self) {
        match self.init_index() {
            Ok(ko) => {
                self.smallest = Key::from(ko.key);
                let mut iter = self.iter(Flag::REVERSED | Flag::NO_CACHE);
                iter.rewind();
                if !iter.valid() {
                    #[cfg(feature = "tracing")]
                    {
                        tracing::error!(target: "table", "failed to initialize biggest key for table: {}", self.path().display());
                    }
                    panic!(
                        "failed to initialize biggest key for table: {}",
                        self.path().display()
                    );
                }
                if let Some(key) = iter.key() {
                    self.biggest = key.to_key();
                }
            }
            Err(e) => {
                // This defer will help gathering debugging info incase initIndex crashes.
                #[cfg(feature = "tracing")]
                scopeguard::defer_on_unwind! {
                    // Get the count of null bytes at the end of file. This is to make sure if there was an
                    // issue with mmap sync or file copy.
                    let mut count = 0;
                    for i in self.mmap.len() - 1 ..=0 {
                        if self.mmap.as_slice()[i] != 0 {
                            break;
                        }
                        count += 1;
                    }

                    {
                        tracing::info!("== Recovering from initIndex crash ==");
                        tracing::info!("File info: [id: {}, size: {}, zeros: {}]", self.id, self.table_size, count);
                        tracing::info!("is_encrypt: {}", self.should_decrypt());

                        let mut read_pos = self.table_size;

                        // Read checksum size.
                        read_pos -= 4;
                        let buf = self.read_no_fail(read_pos, 4);
                        let checksum_len = u32::from_be_bytes(buf.try_into().unwrap());
                        tracing::info!("checksum length: {}", checksum_len);

                        // Read checksum
                        read_pos -= checksum_len as usize;
                        let buf = self.read_no_fail(read_pos, checksum_len as usize);
                        let checksum: Checksum = Marshaller::unmarshal(buf).unwrap();
                        tracing::info!("checksum: {:?}", checksum);

                        // Read index size from the footer.
                        read_pos -= 4;
                        let buf = self.read_no_fail(read_pos, 4);
                        let index_len = u32::from_be_bytes(buf.try_into().unwrap());
                        tracing::info!("index len: {}", index_len);

                        // Read index.
                        read_pos -= 4;
                        self.index_start = read_pos;
                        let index_data = self.read_no_fail(read_pos, self.index_len);
                        tracing::info!("index: {:?}", index_data);
                    }
                };

                #[cfg(feature = "tracing")]
                {
                    tracing::error!(target: "table", info = "fail to init biggest and smallest index for table", err = %e);
                }
                panic!("{}", e)
            }
        }
    }

    /// init_index reads the index and populate the necessary table fields
    fn init_index(&mut self) -> Result<BlockOffset> {
        let mut read_pos = self.table_size;
        // read checksum len from the last 4 bytes.
        read_pos -= 4;
        let buf = self.read_no_fail(read_pos, 4);
        let cks_len = u32::from_be_bytes(buf.try_into().unwrap()) as usize;

        // read checksum
        read_pos -= cks_len;
        let buf = self.read_no_fail(read_pos, cks_len);
        let cks = Checksum::unmarshal(buf)?;

        // read index size from the footer
        read_pos -= 4;
        let buf = self.read_no_fail(read_pos, 4);
        self.index_len = u32::from_be_bytes(buf.try_into().unwrap()) as usize;

        // read index
        read_pos -= self.index_len;
        self.index_start = read_pos;
        let data = self.read_no_fail(read_pos, self.index_len);
        if !data.verify_checksum(cks.sum, Options::checksum(&self.opts)) {
            return Err(Error::ChecksumMismatch);
        }
        let index = self.read_table_index()?;
        let bo = index.offsets[0].clone();
        let has_bloom_filter = !index.bloom_filter.is_empty();
        self.cheap = CheapIndex {
            max_version: index.max_version,
            key_count: index.key_count,
            uncompressed_size: index.uncompressed_size,
            on_disk_size: self.table_size as u32,
            bloom_filter_length: index.bloom_filter.len(),
            offsets_length: index.offsets.len(),
            num_entries: index.key_count as usize,
        };

        if !self.should_decrypt() {
            // If there's no encryption, this points to the mmap'ed buffer.
            self.index = RefCounter::new(index);
        }
        self.has_bloom_filter = has_bloom_filter;
        Ok(bo)
    }

    pub(super) fn fetch_index(&self) -> RefCounter<TableIndex> {
        if !self.should_decrypt() {
            return self.index.clone();
        }

        match self.opts.index_cache() {
            Some(cache) => match cache.get(&self.id) {
                Some(index) => index.as_ref().clone(),
                None => {
                    let index = self.read_table_index().map(RefCounter::new).map_err(|e| {
                            #[cfg(feature = "tracing")]
                            {
                                tracing::error!(target: "table", info = "fail to read table idex", err = %e);
                            }
                            e
                        }).unwrap();
                    #[cfg(feature = "std")]
                    cache.insert(self.id, index.clone(), self.index_len as i64);
                    #[cfg(not(feature = "std"))]
                    cache.insert(self.id, index.clone());
                    index
                }
            },
            None => {
                panic!("Index Cache must be set for encrypted workloads");
            }
        }
    }

    /// read_table_index reads table index from the sst and returns its pb format.
    pub(super) fn read_table_index(&self) -> Result<TableIndex> {
        let buf = self.read_no_fail(self.index_start, self.index_len);

        // Decrypt the table index if it is encrypted.
        if self.should_decrypt() {
            self.decrypt(buf)
                .and_then(|data| TableIndex::unmarshal(data.as_slice()).map_err(From::from))
        } else {
            TableIndex::unmarshal(buf).map_err(From::from)
        }
    }

    #[inline]
    fn should_decrypt(&self) -> bool {
        self.opts.encryption().is_some()
    }

    #[inline]
    fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
        let encryption = self.opts.encryption();
        let block_size = Encryption::BLOCK_SIZE;
        // Last BlockSize bytes of the data is the IV.
        let iv = &data[data.len() - block_size..];
        // reset all bytes are data.
        let data = &data[..data.len() - block_size];

        data.encrypt_to_vec(encryption.secret(), iv, encryption.algorithm())
            .map_err(From::from)
    }
}

#[inline(always)]
fn bytes_to_u32_slice(bytes: &[u8]) -> &[u32] {
    const DUMMY: &[u32] = &[];
    if bytes.is_empty() {
        return DUMMY;
    }

    let len = bytes.len();
    let ptr = bytes.as_ptr();
    // Safety:
    // - This function is not exposed to the public.
    // - bytes is a slice of u32, so it is safe to transmute it to &[u32].
    unsafe { core::slice::from_raw_parts(ptr.cast::<u32>(), len / 4) }
}
