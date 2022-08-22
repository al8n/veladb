#[cfg(feature = "bytes")]
mod datakey;
#[cfg(feature = "bytes")]
pub use datakey::DataKey;

#[cfg(not(feature = "bytes"))]
pub use crate::ty::DataKey;

#[cfg(not(feature = "bytes"))]
impl DataKey {
    pub const fn new() -> Self {
        Self {
            iv: alloc::vec::Vec::new(),
            key_id: 0,
            data: alloc::vec::Vec::new(),
            created_at: 0,
        }
    }
}
