#[derive(Clone, PartialEq, Eq, Hash, ::prost::Message)]
pub struct ManifestChangeSet {
    /// A set of changes that are applied atomically.
    #[prost(message, repeated, tag = "1")]
    pub changes: ::prost::alloc::vec::Vec<ManifestChange>,
}

#[derive(Clone, PartialEq, Eq, Hash, ::prost::Message)]
pub struct ManifestChange {
    /// Table ID.
    #[prost(uint64, tag = "1")]
    pub id: u64,
    #[prost(enumeration = "Operation", tag = "2")]
    pub op: i32,
    /// Only used for CREATE.
    #[prost(uint32, tag = "3")]
    pub level: u32,
    #[prost(uint64, tag = "4")]
    pub key_id: u64,
    #[prost(enumeration = "crate::EncryptionAlgorithm", tag = "5")]
    pub encryption_algo: i32,
    /// Only used for CREATE Op.
    #[prost(uint32, tag = "6")]
    pub compression: u32,
}

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
    pub const fn as_str_name(&self) -> &'static str {
        match self {
            Operation::Create => "CREATE",
            Operation::Delete => "DELETE",
        }
    }
}

impl ManifestChange {
    #[inline]
    pub const fn new_create_change(
        id: u64,
        level: usize,
        key_id: u64,
        encryption_algo: crate::EncryptionAlgorithm,
        compression_ty: u32,
    ) -> Self {
        Self {
            id,
            op: Operation::Create as i32,
            level: level as u32,
            key_id,
            encryption_algo: encryption_algo as i32,
            compression: compression_ty,
        }
    }

    #[inline]
    pub const fn new_delete_change(id: u64, encryption_algo: crate::EncryptionAlgorithm) -> Self {
        Self {
            id,
            op: Operation::Delete as i32,
            level: 0,
            key_id: 0,
            encryption_algo: encryption_algo as i32,
            compression: 0,
        }
    }
}

#[derive(Copy, Clone, Default, Eq, PartialEq, Hash)]
pub struct InvalidManifestChangeOperation;

impl core::fmt::Debug for InvalidManifestChangeOperation {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "invalid manifest change operation")
    }
}

impl core::fmt::Display for InvalidManifestChangeOperation {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "invalid manifest change operation")
    }
}

#[cfg(feature = "std")]
impl std::error::Error for InvalidManifestChangeOperation {}

macro_rules! impl_operation_converter {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl TryFrom<$ty> for Operation {
                type Error = InvalidManifestChangeOperation;

                fn try_from(val: $ty) -> Result<Operation, InvalidManifestChangeOperation> {
                    match val {
                        0 => Ok(Operation::Create),
                        1 => Ok(Operation::Delete),
                        _ => Err(InvalidManifestChangeOperation),
                    }
                }
            }
        )*
    };
}

impl_operation_converter!(i8, i16, i32, i64, isize, i128, u8, u16, u32, u64, usize, u128);
