use kvstructs::bytes::Bytes;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BlockOffset {
    pub key: Bytes,
    pub offset: u32,
    pub len: u32,
}

impl Default for BlockOffset {
    fn default() -> Self {
        Self {
            key: Bytes::new(),
            offset: 0,
            len: 0,
        }
    }
}

impl ::prost::Message for BlockOffset {
    #[allow(unused_variables)]
    fn encode_raw<B>(&self, buf: &mut B)
    where
        B: ::prost::bytes::BufMut,
    {
        if self.key != b"" as &[u8] {
            ::prost::encoding::bytes::encode(1u32, &self.key, buf);
        }
        if self.offset != 0u32 {
            ::prost::encoding::uint32::encode(2u32, &self.offset, buf);
        }
        if self.len != 0u32 {
            ::prost::encoding::uint32::encode(3u32, &self.len, buf);
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
        const STRUCT_NAME: &str = "BlockOffset";
        match tag {
            1u32 => {
                let value = &mut self.key;
                ::prost::encoding::bytes::merge(wire_type, value, buf, ctx).map_err(|mut error| {
                    error.push(STRUCT_NAME, "key");
                    error
                })
            }
            2u32 => {
                let value = &mut self.offset;
                ::prost::encoding::uint32::merge(wire_type, value, buf, ctx).map_err(|mut error| {
                    error.push(STRUCT_NAME, "offset");
                    error
                })
            }
            3u32 => {
                let value = &mut self.len;
                ::prost::encoding::uint32::merge(wire_type, value, buf, ctx).map_err(|mut error| {
                    error.push(STRUCT_NAME, "len");
                    error
                })
            }
            _ => ::prost::encoding::skip_field(wire_type, tag, buf, ctx),
        }
    }

    #[inline]
    fn encoded_len(&self) -> usize {
        (if self.key != b"" as &[u8] {
            ::prost::encoding::bytes::encoded_len(1u32, &self.key)
        } else {
            0
        }) + (if self.offset != 0u32 {
            ::prost::encoding::uint32::encoded_len(2u32, &self.offset)
        } else {
            0
        }) + (if self.len != 0u32 {
            ::prost::encoding::uint32::encoded_len(3u32, &self.len)
        } else {
            0
        })
    }
    fn clear(&mut self) {
        self.key.clear();
        self.offset = 0u32;
        self.len = 0u32;
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct TableIndex {
    pub offsets: ::prost::alloc::vec::Vec<BlockOffset>,
    pub bloom_filter: Bytes,
    pub estimated_size: u32,
    pub max_version: u64,
    pub key_count: u32,
    pub uncompressed_size: u32,
    pub stale_data_size: u32,
}

impl ::prost::Message for TableIndex {
    #[allow(unused_variables)]
    fn encode_raw<B>(&self, buf: &mut B)
    where
        B: ::prost::bytes::BufMut,
    {
        for msg in &self.offsets {
            ::prost::encoding::message::encode(1u32, msg, buf);
        }
        if self.bloom_filter != b"" as &[u8] {
            ::prost::encoding::bytes::encode(2u32, &self.bloom_filter, buf);
        }
        if self.estimated_size != 0u32 {
            ::prost::encoding::uint32::encode(3u32, &self.estimated_size, buf);
        }
        if self.max_version != 0u64 {
            ::prost::encoding::uint64::encode(4u32, &self.max_version, buf);
        }
        if self.key_count != 0u32 {
            ::prost::encoding::uint32::encode(5u32, &self.key_count, buf);
        }
        if self.uncompressed_size != 0u32 {
            ::prost::encoding::uint32::encode(6u32, &self.uncompressed_size, buf);
        }
        if self.stale_data_size != 0u32 {
            ::prost::encoding::uint32::encode(7u32, &self.stale_data_size, buf);
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
        const STRUCT_NAME: &str = "TableIndex";
        match tag {
            1u32 => {
                let value = &mut self.offsets;
                ::prost::encoding::message::merge_repeated(wire_type, value, buf, ctx).map_err(
                    |mut error| {
                        error.push(STRUCT_NAME, "offsets");
                        error
                    },
                )
            }
            2u32 => {
                let value = &mut self.bloom_filter;
                ::prost::encoding::bytes::merge(wire_type, value, buf, ctx).map_err(|mut error| {
                    error.push(STRUCT_NAME, "bloom_filter");
                    error
                })
            }
            3u32 => {
                let value = &mut self.estimated_size;
                ::prost::encoding::uint32::merge(wire_type, value, buf, ctx).map_err(|mut error| {
                    error.push(STRUCT_NAME, "estimated_size");
                    error
                })
            }
            4u32 => {
                let value = &mut self.max_version;
                ::prost::encoding::uint64::merge(wire_type, value, buf, ctx).map_err(|mut error| {
                    error.push(STRUCT_NAME, "max_version");
                    error
                })
            }
            5u32 => {
                let value = &mut self.key_count;
                ::prost::encoding::uint32::merge(wire_type, value, buf, ctx).map_err(|mut error| {
                    error.push(STRUCT_NAME, "key_count");
                    error
                })
            }
            6u32 => {
                let value = &mut self.uncompressed_size;
                ::prost::encoding::uint32::merge(wire_type, value, buf, ctx).map_err(|mut error| {
                    error.push(STRUCT_NAME, "uncompressed_size");
                    error
                })
            }
            7u32 => {
                let value = &mut self.stale_data_size;
                ::prost::encoding::uint32::merge(wire_type, value, buf, ctx).map_err(|mut error| {
                    error.push(STRUCT_NAME, "stale_data_size");
                    error
                })
            }
            _ => ::prost::encoding::skip_field(wire_type, tag, buf, ctx),
        }
    }
    #[inline]
    fn encoded_len(&self) -> usize {
        ::prost::encoding::message::encoded_len_repeated(1u32, &self.offsets)
            + if self.bloom_filter != b"" as &[u8] {
                ::prost::encoding::bytes::encoded_len(2u32, &self.bloom_filter)
            } else {
                0
            }
            + if self.estimated_size != 0u32 {
                ::prost::encoding::uint32::encoded_len(3u32, &self.estimated_size)
            } else {
                0
            }
            + if self.max_version != 0u64 {
                ::prost::encoding::uint64::encoded_len(4u32, &self.max_version)
            } else {
                0
            }
            + if self.key_count != 0u32 {
                ::prost::encoding::uint32::encoded_len(5u32, &self.key_count)
            } else {
                0
            }
            + if self.uncompressed_size != 0u32 {
                ::prost::encoding::uint32::encoded_len(6u32, &self.uncompressed_size)
            } else {
                0
            }
            + if self.stale_data_size != 0u32 {
                ::prost::encoding::uint32::encoded_len(7u32, &self.stale_data_size)
            } else {
                0
            }
    }

    fn clear(&mut self) {
        self.offsets.clear();
        self.bloom_filter.clear();
        self.estimated_size = 0u32;
        self.max_version = 0u64;
        self.key_count = 0u32;
        self.uncompressed_size = 0u32;
        self.stale_data_size = 0u32;
    }
}
