mod datakey;
pub use datakey::DataKey;

mod kv;
pub use kv::*;

mod manifest;
pub use manifest::*;

#[derive(Clone, PartialEq, Eq, Hash, ::prost::Message)]
pub struct Match {
    #[prost(bytes = "vec", tag = "1")]
    pub prefix: ::prost::alloc::vec::Vec<u8>,
    /// Comma separated with dash to represent ranges "1, 2-3, 4-7, 9"
    #[prost(string, tag = "2")]
    pub ignore_bytes: ::prost::alloc::string::String,
}

#[derive(Clone, PartialEq, Eq, Hash, ::prost::Message)]
pub struct BlockOffset {
    #[prost(bytes = "vec", tag = "1")]
    pub key: ::prost::alloc::vec::Vec<u8>,
    #[prost(uint32, tag = "2")]
    pub offset: u32,
    #[prost(uint32, tag = "3")]
    pub len: u32,
}

#[derive(Clone, PartialEq, Eq, Hash, ::prost::Message)]
pub struct TableIndex {
    #[prost(message, repeated, tag = "1")]
    pub offsets: ::prost::alloc::vec::Vec<BlockOffset>,
    #[prost(bytes = "vec", tag = "2")]
    pub bloom_filter: ::prost::alloc::vec::Vec<u8>,
    #[prost(uint32, tag = "3")]
    pub estimated_size: u32,
    #[prost(uint64, tag = "4")]
    pub max_version: u64,
    #[prost(uint32, tag = "5")]
    pub key_count: u32,
    #[prost(uint32, tag = "6")]
    pub uncompressed_size: u32,
    #[prost(uint32, tag = "7")]
    pub stale_data_size: u32,
}
