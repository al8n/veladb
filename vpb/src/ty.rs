#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Kv {
    #[prost(bytes="vec", tag="1")]
    pub key: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes="vec", tag="2")]
    pub value: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes="vec", tag="3")]
    pub user_meta: ::prost::alloc::vec::Vec<u8>,
    #[prost(uint64, tag="4")]
    pub version: u64,
    #[prost(uint64, tag="5")]
    pub expires_at: u64,
    #[prost(bytes="vec", tag="6")]
    pub meta: ::prost::alloc::vec::Vec<u8>,
    /// Stream id is used to identify which stream the KV came from.
    #[prost(uint32, tag="10")]
    pub stream_id: u32,
    /// Stream done is used to indicate end of stream.
    #[prost(bool, tag="11")]
    pub stream_done: bool,
    #[prost(enumeration="kv::Kind", tag="12")]
    pub kind: i32,
}
/// Nested message and enum types in `KV`.
pub mod kv {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum Kind {
        Key = 0,
        DataKey = 1,
        File = 2,
    }
    impl Kind {
        /// String value of the enum field names used in the ProtoBuf definition.
        ///
        /// The values are not transformed in any way and thus are considered stable
        /// (if the ProtoBuf definition does not change) and safe for programmatic use.
        pub fn as_str_name(&self) -> &'static str {
            match self {
                Kind::Key => "KEY",
                Kind::DataKey => "DATA_KEY",
                Kind::File => "FILE",
            }
        }
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct KvList {
    #[prost(message, repeated, tag="1")]
    pub kv: ::prost::alloc::vec::Vec<Kv>,
    /// alloc_ref used internally for memory management.
    #[prost(uint64, tag="10")]
    pub alloc_ref: u64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ManifestChangeSet {
    /// A set of changes that are applied atomically.
    #[prost(message, repeated, tag="1")]
    pub changes: ::prost::alloc::vec::Vec<ManifestChange>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ManifestChange {
    /// Table ID.
    #[prost(uint64, tag="1")]
    pub id: u64,
    #[prost(enumeration="manifest_change::Operation", tag="2")]
    pub op: i32,
    /// Only used for CREATE.
    #[prost(uint32, tag="3")]
    pub level: u32,
    #[prost(uint64, tag="4")]
    pub key_id: u64,
    #[prost(enumeration="EncryptionAlgorithm", tag="5")]
    pub encryption_algo: i32,
    /// Only used for CREATE Op.
    #[prost(uint32, tag="6")]
    pub compression: u32,
}
/// Nested message and enum types in `ManifestChange`.
pub mod manifest_change {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum Operation {
        Create = 0,
        Delete = 1,
    }
    impl Operation {
        /// String value of the enum field names used in the ProtoBuf definition.
        ///
        /// The values are not transformed in any way and thus are considered stable
        /// (if the ProtoBuf definition does not change) and safe for programmatic use.
        pub fn as_str_name(&self) -> &'static str {
            match self {
                Operation::Create => "CREATE",
                Operation::Delete => "DELETE",
            }
        }
    }
}
/// Compression specifies how a block should be compressed.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Compression {
    /// compression algorithm
    #[prost(enumeration="compression::CompressionAlgorithm", tag="1")]
    pub algo: i32,
    /// only for zstd, <= 0 use default(3) compression level, 1 - 21 uses the exact level of zstd compression level, >=22 use the largest compression level supported by zstd.
    #[prost(int32, tag="2")]
    pub level: i32,
}
/// Nested message and enum types in `Compression`.
pub mod compression {
    /// CompressionAlgorithm specifies to use which algorithm to compress a block.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum CompressionAlgorithm {
        /// None mode indicates that a block is not compressed.
        None = 0,
        /// Snappy mode indicates that a block is compressed using Snappy algorithm.
        Snappy = 1,
        /// ZSTD mode indicates that a block is compressed using ZSTD algorithm.
        /// ZSTD,
        Zstd = 2,
        /// Lz4 mode indicates that a block is compressed using lz4 algorithm. 
        Lz4 = 3,
    }
    impl CompressionAlgorithm {
        /// String value of the enum field names used in the ProtoBuf definition.
        ///
        /// The values are not transformed in any way and thus are considered stable
        /// (if the ProtoBuf definition does not change) and safe for programmatic use.
        pub fn as_str_name(&self) -> &'static str {
            match self {
                CompressionAlgorithm::None => "None",
                CompressionAlgorithm::Snappy => "Snappy",
                CompressionAlgorithm::Zstd => "Zstd",
                CompressionAlgorithm::Lz4 => "Lz4",
            }
        }
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Checksum {
    /// For storing type of Checksum algorithm used
    #[prost(enumeration="checksum::ChecksumAlgorithm", tag="1")]
    pub algo: i32,
    #[prost(uint64, tag="2")]
    pub sum: u64,
}
/// Nested message and enum types in `Checksum`.
pub mod checksum {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum ChecksumAlgorithm {
        Crc32c = 0,
        XxHash64 = 1,
        SeaHash = 2,
    }
    impl ChecksumAlgorithm {
        /// String value of the enum field names used in the ProtoBuf definition.
        ///
        /// The values are not transformed in any way and thus are considered stable
        /// (if the ProtoBuf definition does not change) and safe for programmatic use.
        pub fn as_str_name(&self) -> &'static str {
            match self {
                ChecksumAlgorithm::Crc32c => "CRC32C",
                ChecksumAlgorithm::XxHash64 => "XXHash64",
                ChecksumAlgorithm::SeaHash => "SeaHash",
            }
        }
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DataKey {
    #[prost(uint64, tag="1")]
    pub key_id: u64,
    #[prost(bytes="vec", tag="2")]
    pub data: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes="vec", tag="3")]
    pub iv: ::prost::alloc::vec::Vec<u8>,
    #[prost(uint64, tag="4")]
    pub created_at: u64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Match {
    #[prost(bytes="vec", tag="1")]
    pub prefix: ::prost::alloc::vec::Vec<u8>,
    /// Comma separated with dash to represent ranges "1, 2-3, 4-7, 9"
    #[prost(string, tag="2")]
    pub ignore_bytes: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BlockOffset {
    #[prost(bytes="vec", tag="1")]
    pub key: ::prost::alloc::vec::Vec<u8>,
    #[prost(uint32, tag="2")]
    pub offset: u32,
    #[prost(uint32, tag="3")]
    pub len: u32,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TableIndex {
    #[prost(message, repeated, tag="1")]
    pub offsets: ::prost::alloc::vec::Vec<BlockOffset>,
    #[prost(bytes="vec", tag="2")]
    pub bloom_filter: ::prost::alloc::vec::Vec<u8>,
    #[prost(uint32, tag="3")]
    pub estimated_size: u32,
    #[prost(uint64, tag="4")]
    pub max_version: u64,
    #[prost(uint32, tag="5")]
    pub key_count: u32,
    #[prost(uint32, tag="6")]
    pub uncompressed_size: u32,
    #[prost(uint32, tag="7")]
    pub stale_data_size: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum EncryptionAlgorithm {
    None = 0,
    Aes = 1,
}
impl EncryptionAlgorithm {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            EncryptionAlgorithm::None => "None",
            EncryptionAlgorithm::Aes => "Aes",
        }
    }
}
