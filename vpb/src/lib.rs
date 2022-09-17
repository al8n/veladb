#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(docsrs, allow(unused_attributes))]
#![forbid(unsafe_code)]

extern crate alloc;
use alloc::vec::Vec;

mod types;
pub use types::*;

pub use kvstructs;
pub use prost;

/// Compression/Decompression
pub mod compression;
pub use compression::{Compression, CompressionAlgorithm};

/// Encryption/Decryption
pub mod encrypt;
pub use encrypt::{Encryption, EncryptionAlgorithm, IV};

/// Checksum calculation.
pub mod checksum;
pub use checksum::{Checksum, ChecksumAlgorithm};

/// Proto buf encode/decode
pub trait Marshaller {
    /// Encode to proto buf
    fn marshal(&self) -> Vec<u8>
    where
        Self: prost::Message + Default,
    {
        self.encode_to_vec()
    }

    /// Decode from proto buf
    fn unmarshal(data: &[u8]) -> Result<Self, prost::DecodeError>
    where
        Self: prost::Message + Default + Sized,
    {
        prost::Message::decode(data)
    }
}

impl<T> Marshaller for T {}

macro_rules! impl_type {
    ($($ty: ty), +$(,)?) => {
        $(
            impl $ty {
                pub fn new() -> Self {
                    Self::default()
                }
            }
        )*
    };
}

impl_type! {
    ManifestChange, ManifestChangeSet, Match, Kv, KvList, BlockOffset, TableIndex,
}
