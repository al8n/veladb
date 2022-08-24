use kvstructs::bytes::Bytes;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataKey {
    pub key_id: u64,
    pub data: Bytes,
    pub iv: Bytes,
    pub created_at: u64,
}

impl Default for DataKey {
    fn default() -> Self {
        Self::new()
    }
}

impl DataKey {
    pub const fn new() -> Self {
        Self {
            key_id: 0,
            data: Bytes::new(),
            iv: Bytes::new(),
            created_at: 0,
        }
    }
}

impl prost::Message for DataKey {
    #[allow(unused_variables)]
    fn encode_raw<B>(&self, buf: &mut B)
    where
        B: ::prost::bytes::BufMut,
    {
        if self.key_id != 0u64 {
            ::prost::encoding::uint64::encode(1u32, &self.key_id, buf);
        }
        if self.data != b"" as &[u8] {
            ::prost::encoding::bytes::encode(2u32, &self.data, buf);
        }
        if self.iv != b"" as &[u8] {
            ::prost::encoding::bytes::encode(3u32, &self.iv, buf);
        }
        if self.created_at != 0u64 {
            ::prost::encoding::uint64::encode(4u32, &self.created_at, buf);
        }
    }
    #[allow(unused_variables)]
    fn merge_field<B>(
        &mut self,
        tag: u32,
        wire_type: ::prost::encoding::WireType,
        buf: &mut B,
        ctx: ::prost::encoding::DecodeContext,
    ) -> ::core::result::Result<(), ::prost::DecodeError>
    where
        B: ::prost::bytes::Buf,
    {
        const STRUCT_NAME: &str = "DataKey";
        match tag {
            1u32 => {
                let value = &mut self.key_id;
                ::prost::encoding::uint64::merge(wire_type, value, buf, ctx).map_err(|mut error| {
                    error.push(STRUCT_NAME, "key_id");
                    error
                })
            }
            2u32 => {
                let value = &mut self.data;
                ::prost::encoding::bytes::merge(wire_type, value, buf, ctx).map_err(|mut error| {
                    error.push(STRUCT_NAME, "data");
                    error
                })
            }
            3u32 => {
                let value = &mut self.iv;
                ::prost::encoding::bytes::merge(wire_type, value, buf, ctx).map_err(|mut error| {
                    error.push(STRUCT_NAME, "iv");
                    error
                })
            }
            4u32 => {
                let value = &mut self.created_at;
                ::prost::encoding::uint64::merge(wire_type, value, buf, ctx).map_err(|mut error| {
                    error.push(STRUCT_NAME, "created_at");
                    error
                })
            }
            _ => ::prost::encoding::skip_field(wire_type, tag, buf, ctx),
        }
    }
    #[inline]
    fn encoded_len(&self) -> usize {
        (if self.key_id != 0u64 {
            ::prost::encoding::uint64::encoded_len(1u32, &self.key_id)
        } else {
            0
        }) + (if self.data != b"" as &[u8] {
            ::prost::encoding::bytes::encoded_len(2u32, &self.data)
        } else {
            0
        }) + (if self.iv != b"" as &[u8] {
            ::prost::encoding::bytes::encoded_len(3u32, &self.iv)
        } else {
            0
        }) + (if self.created_at != 0u64 {
            ::prost::encoding::uint64::encoded_len(4u32, &self.created_at)
        } else {
            0
        })
    }
    fn clear(&mut self) {
        self.key_id = 0u64;
        self.data.clear();
        self.iv.clear();
        self.created_at = 0u64;
    }
}
