/// ValuePointer points to the value in value log
#[derive(Default, Copy, Clone, Eq, PartialEq, Debug)]
#[repr(C)]
pub struct ValuePointer {
    /// value log fid
    pub fid: u32,
    /// value len
    pub len: u32,
    /// offset of value in the value log
    pub offset: u32,
}

const VP_SIZE: usize = core::mem::size_of::<ValuePointer>();

impl ValuePointer {
    /// Encodes Pointer into `[u8; 12]`.
    pub fn encode(&self) -> [u8; VP_SIZE] {
        let mut data = [0u8; VP_SIZE];
        data[..4].copy_from_slice(u32::to_be_bytes(self.fid).as_ref());
        data[4..8].copy_from_slice(u32::to_be_bytes(self.len).as_ref());
        data[8..].copy_from_slice(u32::to_be_bytes(self.offset).as_ref());
        data
    }

    /// Returns if the value pointer is zero
    #[inline]
    pub fn is_zero(&self) -> bool {
        self.fid == 0 && self.offset == 0 && self.len == 0
    }

    /// Decodes the value pointer into the provided byte buffer.
    #[inline]
    pub fn decode(data: &[u8]) -> Self {
        let fid = u32::from_be_bytes(data[..4].try_into().unwrap());
        let len = u32::from_be_bytes(data[4..8].try_into().unwrap());
        let offset = u32::from_be_bytes(data[8..].try_into().unwrap());
        Self { fid, len, offset }
    }
}

impl PartialOrd<Self> for ValuePointer {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ValuePointer {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        if self.fid != other.fid {
            return self.fid.cmp(&other.fid);
        }

        if self.offset != other.offset {
            return self.offset.cmp(&other.offset);
        }

        self.len.cmp(&other.len)
    }
}
