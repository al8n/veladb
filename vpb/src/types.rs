mod datakey;
pub use datakey::DataKey;

mod kv;
pub use kv::*;

mod manifest;
pub use manifest::*;

mod table_index;
pub use table_index::*;

#[derive(Clone, PartialEq, Eq, Hash, ::prost::Message)]
pub struct Match {
    #[prost(bytes = "vec", tag = "1")]
    pub prefix: ::prost::alloc::vec::Vec<u8>,
    /// Comma separated with dash to represent ranges "1, 2-3, 4-7, 9"
    #[prost(string, tag = "2")]
    pub ignore_bytes: ::prost::alloc::string::String,
}
