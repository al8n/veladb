#[derive(Clone, PartialEq, Eq, Hash, ::prost::Message)]
pub struct Kv {
    #[prost(bytes = "vec", tag = "1")]
    pub key: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    pub value: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "3")]
    pub user_meta: ::prost::alloc::vec::Vec<u8>,
    #[prost(uint64, tag = "4")]
    pub version: u64,
    #[prost(uint64, tag = "5")]
    pub expires_at: u64,
    #[prost(bytes = "vec", tag = "6")]
    pub meta: ::prost::alloc::vec::Vec<u8>,
    /// Stream id is used to identify which stream the KV came from.
    #[prost(uint32, tag = "10")]
    pub stream_id: u32,
    /// Stream done is used to indicate end of stream.
    #[prost(bool, tag = "11")]
    pub stream_done: bool,
    #[prost(enumeration = "Kind", tag = "12")]
    pub kind: i32,
}

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
    pub const fn as_str_name(&self) -> &'static str {
        match self {
            Kind::Key => "KEY",
            Kind::DataKey => "DATA_KEY",
            Kind::File => "FILE",
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash, ::prost::Message)]
pub struct KvList {
    #[prost(message, repeated, tag = "1")]
    pub kv: ::prost::alloc::vec::Vec<Kv>,
    /// alloc_ref used internally for memory management.
    #[prost(uint64, tag = "10")]
    pub alloc_ref: u64,
}

#[derive(Copy, Clone, Default, Eq, PartialEq, Hash)]
pub struct InvalidKvKind;

impl core::fmt::Debug for InvalidKvKind {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "invalid key-value message kind")
    }
}

impl core::fmt::Display for InvalidKvKind {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "invalid key-value message kind")
    }
}

#[cfg(feature = "std")]
impl std::error::Error for InvalidKvKind {}

macro_rules! impl_kv_kind_converter {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl TryFrom<$ty> for Kind {
                type Error = InvalidKvKind;

                fn try_from(val: $ty) -> Result<Kind, InvalidKvKind> {
                    match val {
                        0 => Ok(Kind::Key),
                        1 => Ok(Kind::DataKey),
                        2 => Ok(Kind::File),
                        _ => Err(InvalidKvKind),
                    }
                }
            }
        )*
    };
}

impl_kv_kind_converter!(i8, i16, i32, i64, isize, i128, u8, u16, u32, u64, usize, u128);
