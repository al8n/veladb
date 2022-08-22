mod ty {
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
        #[prost(enumeration = "kv::Kind", tag = "12")]
        pub kind: i32,
    }
    #[automatically_derived]
    impl ::core::clone::Clone for Kv {
        #[inline]
        fn clone(&self) -> Kv {
            Kv {
                key: ::core::clone::Clone::clone(&self.key),
                value: ::core::clone::Clone::clone(&self.value),
                user_meta: ::core::clone::Clone::clone(&self.user_meta),
                version: ::core::clone::Clone::clone(&self.version),
                expires_at: ::core::clone::Clone::clone(&self.expires_at),
                meta: ::core::clone::Clone::clone(&self.meta),
                stream_id: ::core::clone::Clone::clone(&self.stream_id),
                stream_done: ::core::clone::Clone::clone(&self.stream_done),
                kind: ::core::clone::Clone::clone(&self.kind),
            }
        }
    }
    impl ::core::marker::StructuralPartialEq for Kv {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for Kv {
        #[inline]
        fn eq(&self, other: &Kv) -> bool {
            self.key == other.key
                && self.value == other.value
                && self.user_meta == other.user_meta
                && self.version == other.version
                && self.expires_at == other.expires_at
                && self.meta == other.meta
                && self.stream_id == other.stream_id
                && self.stream_done == other.stream_done
                && self.kind == other.kind
        }
        #[inline]
        fn ne(&self, other: &Kv) -> bool {
            self.key != other.key
                || self.value != other.value
                || self.user_meta != other.user_meta
                || self.version != other.version
                || self.expires_at != other.expires_at
                || self.meta != other.meta
                || self.stream_id != other.stream_id
                || self.stream_done != other.stream_done
                || self.kind != other.kind
        }
    }
    impl ::prost::Message for Kv {
        #[allow(unused_variables)]
        fn encode_raw<B>(&self, buf: &mut B)
        where
            B: ::prost::bytes::BufMut,
        {
            if self.key != b"" as &[u8] {
                ::prost::encoding::bytes::encode(1u32, &self.key, buf);
            }
            if self.value != b"" as &[u8] {
                ::prost::encoding::bytes::encode(2u32, &self.value, buf);
            }
            if self.user_meta != b"" as &[u8] {
                ::prost::encoding::bytes::encode(3u32, &self.user_meta, buf);
            }
            if self.version != 0u64 {
                ::prost::encoding::uint64::encode(4u32, &self.version, buf);
            }
            if self.expires_at != 0u64 {
                ::prost::encoding::uint64::encode(5u32, &self.expires_at, buf);
            }
            if self.meta != b"" as &[u8] {
                ::prost::encoding::bytes::encode(6u32, &self.meta, buf);
            }
            if self.stream_id != 0u32 {
                ::prost::encoding::uint32::encode(10u32, &self.stream_id, buf);
            }
            if self.stream_done != false {
                ::prost::encoding::bool::encode(11u32, &self.stream_done, buf);
            }
            if self.kind != kv::Kind::default() as i32 {
                ::prost::encoding::int32::encode(12u32, &self.kind, buf);
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
            const STRUCT_NAME: &'static str = "Kv";
            match tag {
                1u32 => {
                    let mut value = &mut self.key;
                    ::prost::encoding::bytes::merge(wire_type, value, buf, ctx).map_err(
                        |mut error| {
                            error.push(STRUCT_NAME, "key");
                            error
                        },
                    )
                }
                2u32 => {
                    let mut value = &mut self.value;
                    ::prost::encoding::bytes::merge(wire_type, value, buf, ctx).map_err(
                        |mut error| {
                            error.push(STRUCT_NAME, "value");
                            error
                        },
                    )
                }
                3u32 => {
                    let mut value = &mut self.user_meta;
                    ::prost::encoding::bytes::merge(wire_type, value, buf, ctx).map_err(
                        |mut error| {
                            error.push(STRUCT_NAME, "user_meta");
                            error
                        },
                    )
                }
                4u32 => {
                    let mut value = &mut self.version;
                    ::prost::encoding::uint64::merge(wire_type, value, buf, ctx).map_err(
                        |mut error| {
                            error.push(STRUCT_NAME, "version");
                            error
                        },
                    )
                }
                5u32 => {
                    let mut value = &mut self.expires_at;
                    ::prost::encoding::uint64::merge(wire_type, value, buf, ctx).map_err(
                        |mut error| {
                            error.push(STRUCT_NAME, "expires_at");
                            error
                        },
                    )
                }
                6u32 => {
                    let mut value = &mut self.meta;
                    ::prost::encoding::bytes::merge(wire_type, value, buf, ctx).map_err(
                        |mut error| {
                            error.push(STRUCT_NAME, "meta");
                            error
                        },
                    )
                }
                10u32 => {
                    let mut value = &mut self.stream_id;
                    ::prost::encoding::uint32::merge(wire_type, value, buf, ctx).map_err(
                        |mut error| {
                            error.push(STRUCT_NAME, "stream_id");
                            error
                        },
                    )
                }
                11u32 => {
                    let mut value = &mut self.stream_done;
                    ::prost::encoding::bool::merge(wire_type, value, buf, ctx).map_err(
                        |mut error| {
                            error.push(STRUCT_NAME, "stream_done");
                            error
                        },
                    )
                }
                12u32 => {
                    let mut value = &mut self.kind;
                    ::prost::encoding::int32::merge(wire_type, value, buf, ctx).map_err(
                        |mut error| {
                            error.push(STRUCT_NAME, "kind");
                            error
                        },
                    )
                }
                _ => ::prost::encoding::skip_field(wire_type, tag, buf, ctx),
            }
        }
        #[inline]
        fn encoded_len(&self) -> usize {
            0 + if self.key != b"" as &[u8] {
                ::prost::encoding::bytes::encoded_len(1u32, &self.key)
            } else {
                0
            } + if self.value != b"" as &[u8] {
                ::prost::encoding::bytes::encoded_len(2u32, &self.value)
            } else {
                0
            } + if self.user_meta != b"" as &[u8] {
                ::prost::encoding::bytes::encoded_len(3u32, &self.user_meta)
            } else {
                0
            } + if self.version != 0u64 {
                ::prost::encoding::uint64::encoded_len(4u32, &self.version)
            } else {
                0
            } + if self.expires_at != 0u64 {
                ::prost::encoding::uint64::encoded_len(5u32, &self.expires_at)
            } else {
                0
            } + if self.meta != b"" as &[u8] {
                ::prost::encoding::bytes::encoded_len(6u32, &self.meta)
            } else {
                0
            } + if self.stream_id != 0u32 {
                ::prost::encoding::uint32::encoded_len(10u32, &self.stream_id)
            } else {
                0
            } + if self.stream_done != false {
                ::prost::encoding::bool::encoded_len(11u32, &self.stream_done)
            } else {
                0
            } + if self.kind != kv::Kind::default() as i32 {
                ::prost::encoding::int32::encoded_len(12u32, &self.kind)
            } else {
                0
            }
        }
        fn clear(&mut self) {
            self.key.clear();
            self.value.clear();
            self.user_meta.clear();
            self.version = 0u64;
            self.expires_at = 0u64;
            self.meta.clear();
            self.stream_id = 0u32;
            self.stream_done = false;
            self.kind = kv::Kind::default() as i32;
        }
    }
    impl ::core::default::Default for Kv {
        fn default() -> Self {
            Kv {
                key: ::core::default::Default::default(),
                value: ::core::default::Default::default(),
                user_meta: ::core::default::Default::default(),
                version: 0u64,
                expires_at: 0u64,
                meta: ::core::default::Default::default(),
                stream_id: 0u32,
                stream_done: false,
                kind: kv::Kind::default() as i32,
            }
        }
    }
    impl ::core::fmt::Debug for Kv {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            let mut builder = f.debug_struct("Kv");
            let builder = {
                let wrapper = {
                    fn ScalarWrapper<T>(v: T) -> T {
                        v
                    }
                    ScalarWrapper(&self.key)
                };
                builder.field("key", &wrapper)
            };
            let builder = {
                let wrapper = {
                    fn ScalarWrapper<T>(v: T) -> T {
                        v
                    }
                    ScalarWrapper(&self.value)
                };
                builder.field("value", &wrapper)
            };
            let builder = {
                let wrapper = {
                    fn ScalarWrapper<T>(v: T) -> T {
                        v
                    }
                    ScalarWrapper(&self.user_meta)
                };
                builder.field("user_meta", &wrapper)
            };
            let builder = {
                let wrapper = {
                    fn ScalarWrapper<T>(v: T) -> T {
                        v
                    }
                    ScalarWrapper(&self.version)
                };
                builder.field("version", &wrapper)
            };
            let builder = {
                let wrapper = {
                    fn ScalarWrapper<T>(v: T) -> T {
                        v
                    }
                    ScalarWrapper(&self.expires_at)
                };
                builder.field("expires_at", &wrapper)
            };
            let builder = {
                let wrapper = {
                    fn ScalarWrapper<T>(v: T) -> T {
                        v
                    }
                    ScalarWrapper(&self.meta)
                };
                builder.field("meta", &wrapper)
            };
            let builder = {
                let wrapper = {
                    fn ScalarWrapper<T>(v: T) -> T {
                        v
                    }
                    ScalarWrapper(&self.stream_id)
                };
                builder.field("stream_id", &wrapper)
            };
            let builder = {
                let wrapper = {
                    fn ScalarWrapper<T>(v: T) -> T {
                        v
                    }
                    ScalarWrapper(&self.stream_done)
                };
                builder.field("stream_done", &wrapper)
            };
            let builder = {
                let wrapper = {
                    struct ScalarWrapper<'a>(&'a i32);
                    impl<'a> ::core::fmt::Debug for ScalarWrapper<'a> {
                        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                            match kv::Kind::from_i32(*self.0) {
                                None => ::core::fmt::Debug::fmt(&self.0, f),
                                Some(en) => ::core::fmt::Debug::fmt(&en, f),
                            }
                        }
                    }
                    ScalarWrapper(&self.kind)
                };
                builder.field("kind", &wrapper)
            };
            builder.finish()
        }
    }
    #[allow(dead_code)]
    impl Kv {
        ///Returns the enum value of `kind`, or the default if the field is set to an invalid enum value.
        pub fn kind(&self) -> kv::Kind {
            kv::Kind::from_i32(self.kind).unwrap_or(kv::Kind::default())
        }
        ///Sets `kind` to the provided enum value.
        pub fn set_kind(&mut self, value: kv::Kind) {
            self.kind = value as i32;
        }
    }
    /// Nested message and enum types in `KV`.
    pub mod kv {
        #[repr(i32)]
        pub enum Kind {
            Key = 0,
            DataKey = 1,
            File = 2,
        }
        #[automatically_derived]
        impl ::core::clone::Clone for Kind {
            #[inline]
            fn clone(&self) -> Kind {
                *self
            }
        }
        #[automatically_derived]
        impl ::core::marker::Copy for Kind {}
        #[automatically_derived]
        impl ::core::fmt::Debug for Kind {
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                match self {
                    Kind::Key => ::core::fmt::Formatter::write_str(f, "Key"),
                    Kind::DataKey => ::core::fmt::Formatter::write_str(f, "DataKey"),
                    Kind::File => ::core::fmt::Formatter::write_str(f, "File"),
                }
            }
        }
        impl ::core::marker::StructuralPartialEq for Kind {}
        #[automatically_derived]
        impl ::core::cmp::PartialEq for Kind {
            #[inline]
            fn eq(&self, other: &Kind) -> bool {
                let __self_tag = ::core::intrinsics::discriminant_value(self);
                let __arg1_tag = ::core::intrinsics::discriminant_value(other);
                __self_tag == __arg1_tag
            }
        }
        impl ::core::marker::StructuralEq for Kind {}
        #[automatically_derived]
        impl ::core::cmp::Eq for Kind {
            #[inline]
            #[doc(hidden)]
            #[no_coverage]
            fn assert_receiver_is_total_eq(&self) -> () {}
        }
        #[automatically_derived]
        impl ::core::hash::Hash for Kind {
            fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {
                let __self_tag = ::core::intrinsics::discriminant_value(self);
                ::core::hash::Hash::hash(&__self_tag, state)
            }
        }
        #[automatically_derived]
        impl ::core::cmp::PartialOrd for Kind {
            #[inline]
            fn partial_cmp(&self, other: &Kind) -> ::core::option::Option<::core::cmp::Ordering> {
                let __self_tag = ::core::intrinsics::discriminant_value(self);
                let __arg1_tag = ::core::intrinsics::discriminant_value(other);
                ::core::cmp::PartialOrd::partial_cmp(&__self_tag, &__arg1_tag)
            }
        }
        #[automatically_derived]
        impl ::core::cmp::Ord for Kind {
            #[inline]
            fn cmp(&self, other: &Kind) -> ::core::cmp::Ordering {
                let __self_tag = ::core::intrinsics::discriminant_value(self);
                let __arg1_tag = ::core::intrinsics::discriminant_value(other);
                ::core::cmp::Ord::cmp(&__self_tag, &__arg1_tag)
            }
        }
        impl Kind {
            ///Returns `true` if `value` is a variant of `Kind`.
            pub fn is_valid(value: i32) -> bool {
                match value {
                    0 => true,
                    1 => true,
                    2 => true,
                    _ => false,
                }
            }
            ///Converts an `i32` to a `Kind`, or `None` if `value` is not a valid variant.
            pub fn from_i32(value: i32) -> ::core::option::Option<Kind> {
                match value {
                    0 => ::core::option::Option::Some(Kind::Key),
                    1 => ::core::option::Option::Some(Kind::DataKey),
                    2 => ::core::option::Option::Some(Kind::File),
                    _ => ::core::option::Option::None,
                }
            }
        }
        impl ::core::default::Default for Kind {
            fn default() -> Kind {
                Kind::Key
            }
        }
        impl ::core::convert::From<Kind> for i32 {
            fn from(value: Kind) -> i32 {
                value as i32
            }
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
    pub struct KvList {
        #[prost(message, repeated, tag = "1")]
        pub kv: ::prost::alloc::vec::Vec<Kv>,
        /// alloc_ref used internally for memory management.
        #[prost(uint64, tag = "10")]
        pub alloc_ref: u64,
    }
    #[automatically_derived]
    impl ::core::clone::Clone for KvList {
        #[inline]
        fn clone(&self) -> KvList {
            KvList {
                kv: ::core::clone::Clone::clone(&self.kv),
                alloc_ref: ::core::clone::Clone::clone(&self.alloc_ref),
            }
        }
    }
    impl ::core::marker::StructuralPartialEq for KvList {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for KvList {
        #[inline]
        fn eq(&self, other: &KvList) -> bool {
            self.kv == other.kv && self.alloc_ref == other.alloc_ref
        }
        #[inline]
        fn ne(&self, other: &KvList) -> bool {
            self.kv != other.kv || self.alloc_ref != other.alloc_ref
        }
    }
    impl ::prost::Message for KvList {
        #[allow(unused_variables)]
        fn encode_raw<B>(&self, buf: &mut B)
        where
            B: ::prost::bytes::BufMut,
        {
            for msg in &self.kv {
                ::prost::encoding::message::encode(1u32, msg, buf);
            }
            if self.alloc_ref != 0u64 {
                ::prost::encoding::uint64::encode(10u32, &self.alloc_ref, buf);
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
            const STRUCT_NAME: &'static str = "KvList";
            match tag {
                1u32 => {
                    let mut value = &mut self.kv;
                    ::prost::encoding::message::merge_repeated(wire_type, value, buf, ctx).map_err(
                        |mut error| {
                            error.push(STRUCT_NAME, "kv");
                            error
                        },
                    )
                }
                10u32 => {
                    let mut value = &mut self.alloc_ref;
                    ::prost::encoding::uint64::merge(wire_type, value, buf, ctx).map_err(
                        |mut error| {
                            error.push(STRUCT_NAME, "alloc_ref");
                            error
                        },
                    )
                }
                _ => ::prost::encoding::skip_field(wire_type, tag, buf, ctx),
            }
        }
        #[inline]
        fn encoded_len(&self) -> usize {
            0 + ::prost::encoding::message::encoded_len_repeated(1u32, &self.kv)
                + if self.alloc_ref != 0u64 {
                    ::prost::encoding::uint64::encoded_len(10u32, &self.alloc_ref)
                } else {
                    0
                }
        }
        fn clear(&mut self) {
            self.kv.clear();
            self.alloc_ref = 0u64;
        }
    }
    impl ::core::default::Default for KvList {
        fn default() -> Self {
            KvList {
                kv: ::core::default::Default::default(),
                alloc_ref: 0u64,
            }
        }
    }
    impl ::core::fmt::Debug for KvList {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            let mut builder = f.debug_struct("KvList");
            let builder = {
                let wrapper = &self.kv;
                builder.field("kv", &wrapper)
            };
            let builder = {
                let wrapper = {
                    fn ScalarWrapper<T>(v: T) -> T {
                        v
                    }
                    ScalarWrapper(&self.alloc_ref)
                };
                builder.field("alloc_ref", &wrapper)
            };
            builder.finish()
        }
    }
    pub struct ManifestChangeSet {
        /// A set of changes that are applied atomically.
        #[prost(message, repeated, tag = "1")]
        pub changes: ::prost::alloc::vec::Vec<ManifestChange>,
    }
    #[automatically_derived]
    impl ::core::clone::Clone for ManifestChangeSet {
        #[inline]
        fn clone(&self) -> ManifestChangeSet {
            ManifestChangeSet {
                changes: ::core::clone::Clone::clone(&self.changes),
            }
        }
    }
    impl ::core::marker::StructuralPartialEq for ManifestChangeSet {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for ManifestChangeSet {
        #[inline]
        fn eq(&self, other: &ManifestChangeSet) -> bool {
            self.changes == other.changes
        }
        #[inline]
        fn ne(&self, other: &ManifestChangeSet) -> bool {
            self.changes != other.changes
        }
    }
    impl ::prost::Message for ManifestChangeSet {
        #[allow(unused_variables)]
        fn encode_raw<B>(&self, buf: &mut B)
        where
            B: ::prost::bytes::BufMut,
        {
            for msg in &self.changes {
                ::prost::encoding::message::encode(1u32, msg, buf);
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
            const STRUCT_NAME: &'static str = "ManifestChangeSet";
            match tag {
                1u32 => {
                    let mut value = &mut self.changes;
                    ::prost::encoding::message::merge_repeated(wire_type, value, buf, ctx).map_err(
                        |mut error| {
                            error.push(STRUCT_NAME, "changes");
                            error
                        },
                    )
                }
                _ => ::prost::encoding::skip_field(wire_type, tag, buf, ctx),
            }
        }
        #[inline]
        fn encoded_len(&self) -> usize {
            0 + ::prost::encoding::message::encoded_len_repeated(1u32, &self.changes)
        }
        fn clear(&mut self) {
            self.changes.clear();
        }
    }
    impl ::core::default::Default for ManifestChangeSet {
        fn default() -> Self {
            ManifestChangeSet {
                changes: ::core::default::Default::default(),
            }
        }
    }
    impl ::core::fmt::Debug for ManifestChangeSet {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            let mut builder = f.debug_struct("ManifestChangeSet");
            let builder = {
                let wrapper = &self.changes;
                builder.field("changes", &wrapper)
            };
            builder.finish()
        }
    }
    pub struct ManifestChange {
        /// Table ID.
        #[prost(uint64, tag = "1")]
        pub id: u64,
        #[prost(enumeration = "manifest_change::Operation", tag = "2")]
        pub op: i32,
        /// Only used for CREATE.
        #[prost(uint32, tag = "3")]
        pub level: u32,
        #[prost(uint64, tag = "4")]
        pub key_id: u64,
        #[prost(enumeration = "EncryptionAlgorithm", tag = "5")]
        pub encryption_algo: i32,
        /// Only used for CREATE Op.
        #[prost(uint32, tag = "6")]
        pub compression: u32,
    }
    #[automatically_derived]
    impl ::core::clone::Clone for ManifestChange {
        #[inline]
        fn clone(&self) -> ManifestChange {
            ManifestChange {
                id: ::core::clone::Clone::clone(&self.id),
                op: ::core::clone::Clone::clone(&self.op),
                level: ::core::clone::Clone::clone(&self.level),
                key_id: ::core::clone::Clone::clone(&self.key_id),
                encryption_algo: ::core::clone::Clone::clone(&self.encryption_algo),
                compression: ::core::clone::Clone::clone(&self.compression),
            }
        }
    }
    impl ::core::marker::StructuralPartialEq for ManifestChange {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for ManifestChange {
        #[inline]
        fn eq(&self, other: &ManifestChange) -> bool {
            self.id == other.id
                && self.op == other.op
                && self.level == other.level
                && self.key_id == other.key_id
                && self.encryption_algo == other.encryption_algo
                && self.compression == other.compression
        }
        #[inline]
        fn ne(&self, other: &ManifestChange) -> bool {
            self.id != other.id
                || self.op != other.op
                || self.level != other.level
                || self.key_id != other.key_id
                || self.encryption_algo != other.encryption_algo
                || self.compression != other.compression
        }
    }
    impl ::prost::Message for ManifestChange {
        #[allow(unused_variables)]
        fn encode_raw<B>(&self, buf: &mut B)
        where
            B: ::prost::bytes::BufMut,
        {
            if self.id != 0u64 {
                ::prost::encoding::uint64::encode(1u32, &self.id, buf);
            }
            if self.op != manifest_change::Operation::default() as i32 {
                ::prost::encoding::int32::encode(2u32, &self.op, buf);
            }
            if self.level != 0u32 {
                ::prost::encoding::uint32::encode(3u32, &self.level, buf);
            }
            if self.key_id != 0u64 {
                ::prost::encoding::uint64::encode(4u32, &self.key_id, buf);
            }
            if self.encryption_algo != EncryptionAlgorithm::default() as i32 {
                ::prost::encoding::int32::encode(5u32, &self.encryption_algo, buf);
            }
            if self.compression != 0u32 {
                ::prost::encoding::uint32::encode(6u32, &self.compression, buf);
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
            const STRUCT_NAME: &'static str = "ManifestChange";
            match tag {
                1u32 => {
                    let mut value = &mut self.id;
                    ::prost::encoding::uint64::merge(wire_type, value, buf, ctx).map_err(
                        |mut error| {
                            error.push(STRUCT_NAME, "id");
                            error
                        },
                    )
                }
                2u32 => {
                    let mut value = &mut self.op;
                    ::prost::encoding::int32::merge(wire_type, value, buf, ctx).map_err(
                        |mut error| {
                            error.push(STRUCT_NAME, "op");
                            error
                        },
                    )
                }
                3u32 => {
                    let mut value = &mut self.level;
                    ::prost::encoding::uint32::merge(wire_type, value, buf, ctx).map_err(
                        |mut error| {
                            error.push(STRUCT_NAME, "level");
                            error
                        },
                    )
                }
                4u32 => {
                    let mut value = &mut self.key_id;
                    ::prost::encoding::uint64::merge(wire_type, value, buf, ctx).map_err(
                        |mut error| {
                            error.push(STRUCT_NAME, "key_id");
                            error
                        },
                    )
                }
                5u32 => {
                    let mut value = &mut self.encryption_algo;
                    ::prost::encoding::int32::merge(wire_type, value, buf, ctx).map_err(
                        |mut error| {
                            error.push(STRUCT_NAME, "encryption_algo");
                            error
                        },
                    )
                }
                6u32 => {
                    let mut value = &mut self.compression;
                    ::prost::encoding::uint32::merge(wire_type, value, buf, ctx).map_err(
                        |mut error| {
                            error.push(STRUCT_NAME, "compression");
                            error
                        },
                    )
                }
                _ => ::prost::encoding::skip_field(wire_type, tag, buf, ctx),
            }
        }
        #[inline]
        fn encoded_len(&self) -> usize {
            0 + if self.id != 0u64 {
                ::prost::encoding::uint64::encoded_len(1u32, &self.id)
            } else {
                0
            } + if self.op != manifest_change::Operation::default() as i32 {
                ::prost::encoding::int32::encoded_len(2u32, &self.op)
            } else {
                0
            } + if self.level != 0u32 {
                ::prost::encoding::uint32::encoded_len(3u32, &self.level)
            } else {
                0
            } + if self.key_id != 0u64 {
                ::prost::encoding::uint64::encoded_len(4u32, &self.key_id)
            } else {
                0
            } + if self.encryption_algo != EncryptionAlgorithm::default() as i32 {
                ::prost::encoding::int32::encoded_len(5u32, &self.encryption_algo)
            } else {
                0
            } + if self.compression != 0u32 {
                ::prost::encoding::uint32::encoded_len(6u32, &self.compression)
            } else {
                0
            }
        }
        fn clear(&mut self) {
            self.id = 0u64;
            self.op = manifest_change::Operation::default() as i32;
            self.level = 0u32;
            self.key_id = 0u64;
            self.encryption_algo = EncryptionAlgorithm::default() as i32;
            self.compression = 0u32;
        }
    }
    impl ::core::default::Default for ManifestChange {
        fn default() -> Self {
            ManifestChange {
                id: 0u64,
                op: manifest_change::Operation::default() as i32,
                level: 0u32,
                key_id: 0u64,
                encryption_algo: EncryptionAlgorithm::default() as i32,
                compression: 0u32,
            }
        }
    }
    impl ::core::fmt::Debug for ManifestChange {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            let mut builder = f.debug_struct("ManifestChange");
            let builder = {
                let wrapper = {
                    fn ScalarWrapper<T>(v: T) -> T {
                        v
                    }
                    ScalarWrapper(&self.id)
                };
                builder.field("id", &wrapper)
            };
            let builder = {
                let wrapper = {
                    struct ScalarWrapper<'a>(&'a i32);
                    impl<'a> ::core::fmt::Debug for ScalarWrapper<'a> {
                        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                            match manifest_change::Operation::from_i32(*self.0) {
                                None => ::core::fmt::Debug::fmt(&self.0, f),
                                Some(en) => ::core::fmt::Debug::fmt(&en, f),
                            }
                        }
                    }
                    ScalarWrapper(&self.op)
                };
                builder.field("op", &wrapper)
            };
            let builder = {
                let wrapper = {
                    fn ScalarWrapper<T>(v: T) -> T {
                        v
                    }
                    ScalarWrapper(&self.level)
                };
                builder.field("level", &wrapper)
            };
            let builder = {
                let wrapper = {
                    fn ScalarWrapper<T>(v: T) -> T {
                        v
                    }
                    ScalarWrapper(&self.key_id)
                };
                builder.field("key_id", &wrapper)
            };
            let builder = {
                let wrapper = {
                    struct ScalarWrapper<'a>(&'a i32);
                    impl<'a> ::core::fmt::Debug for ScalarWrapper<'a> {
                        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                            match EncryptionAlgorithm::from_i32(*self.0) {
                                None => ::core::fmt::Debug::fmt(&self.0, f),
                                Some(en) => ::core::fmt::Debug::fmt(&en, f),
                            }
                        }
                    }
                    ScalarWrapper(&self.encryption_algo)
                };
                builder.field("encryption_algo", &wrapper)
            };
            let builder = {
                let wrapper = {
                    fn ScalarWrapper<T>(v: T) -> T {
                        v
                    }
                    ScalarWrapper(&self.compression)
                };
                builder.field("compression", &wrapper)
            };
            builder.finish()
        }
    }
    #[allow(dead_code)]
    impl ManifestChange {
        ///Returns the enum value of `op`, or the default if the field is set to an invalid enum value.
        pub fn op(&self) -> manifest_change::Operation {
            manifest_change::Operation::from_i32(self.op)
                .unwrap_or(manifest_change::Operation::default())
        }
        ///Sets `op` to the provided enum value.
        pub fn set_op(&mut self, value: manifest_change::Operation) {
            self.op = value as i32;
        }
        ///Returns the enum value of `encryption_algo`, or the default if the field is set to an invalid enum value.
        pub fn encryption_algo(&self) -> EncryptionAlgorithm {
            EncryptionAlgorithm::from_i32(self.encryption_algo)
                .unwrap_or(EncryptionAlgorithm::default())
        }
        ///Sets `encryption_algo` to the provided enum value.
        pub fn set_encryption_algo(&mut self, value: EncryptionAlgorithm) {
            self.encryption_algo = value as i32;
        }
    }
    /// Nested message and enum types in `ManifestChange`.
    pub mod manifest_change {
        #[repr(i32)]
        pub enum Operation {
            Create = 0,
            Delete = 1,
        }
        #[automatically_derived]
        impl ::core::clone::Clone for Operation {
            #[inline]
            fn clone(&self) -> Operation {
                *self
            }
        }
        #[automatically_derived]
        impl ::core::marker::Copy for Operation {}
        #[automatically_derived]
        impl ::core::fmt::Debug for Operation {
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                match self {
                    Operation::Create => ::core::fmt::Formatter::write_str(f, "Create"),
                    Operation::Delete => ::core::fmt::Formatter::write_str(f, "Delete"),
                }
            }
        }
        impl ::core::marker::StructuralPartialEq for Operation {}
        #[automatically_derived]
        impl ::core::cmp::PartialEq for Operation {
            #[inline]
            fn eq(&self, other: &Operation) -> bool {
                let __self_tag = ::core::intrinsics::discriminant_value(self);
                let __arg1_tag = ::core::intrinsics::discriminant_value(other);
                __self_tag == __arg1_tag
            }
        }
        impl ::core::marker::StructuralEq for Operation {}
        #[automatically_derived]
        impl ::core::cmp::Eq for Operation {
            #[inline]
            #[doc(hidden)]
            #[no_coverage]
            fn assert_receiver_is_total_eq(&self) -> () {}
        }
        #[automatically_derived]
        impl ::core::hash::Hash for Operation {
            fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {
                let __self_tag = ::core::intrinsics::discriminant_value(self);
                ::core::hash::Hash::hash(&__self_tag, state)
            }
        }
        #[automatically_derived]
        impl ::core::cmp::PartialOrd for Operation {
            #[inline]
            fn partial_cmp(
                &self,
                other: &Operation,
            ) -> ::core::option::Option<::core::cmp::Ordering> {
                let __self_tag = ::core::intrinsics::discriminant_value(self);
                let __arg1_tag = ::core::intrinsics::discriminant_value(other);
                ::core::cmp::PartialOrd::partial_cmp(&__self_tag, &__arg1_tag)
            }
        }
        #[automatically_derived]
        impl ::core::cmp::Ord for Operation {
            #[inline]
            fn cmp(&self, other: &Operation) -> ::core::cmp::Ordering {
                let __self_tag = ::core::intrinsics::discriminant_value(self);
                let __arg1_tag = ::core::intrinsics::discriminant_value(other);
                ::core::cmp::Ord::cmp(&__self_tag, &__arg1_tag)
            }
        }
        impl Operation {
            ///Returns `true` if `value` is a variant of `Operation`.
            pub fn is_valid(value: i32) -> bool {
                match value {
                    0 => true,
                    1 => true,
                    _ => false,
                }
            }
            ///Converts an `i32` to a `Operation`, or `None` if `value` is not a valid variant.
            pub fn from_i32(value: i32) -> ::core::option::Option<Operation> {
                match value {
                    0 => ::core::option::Option::Some(Operation::Create),
                    1 => ::core::option::Option::Some(Operation::Delete),
                    _ => ::core::option::Option::None,
                }
            }
        }
        impl ::core::default::Default for Operation {
            fn default() -> Operation {
                Operation::Create
            }
        }
        impl ::core::convert::From<Operation> for i32 {
            fn from(value: Operation) -> i32 {
                value as i32
            }
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
    pub struct Compression {
        /// compression algorithm
        #[prost(enumeration = "compression::CompressionAlgorithm", tag = "1")]
        pub algo: i32,
        /// only for zstd, <= 0 use default(3) compression level, 1 - 21 uses the exact level of zstd compression level, >=22 use the largest compression level supported by zstd.
        #[prost(int32, tag = "2")]
        pub level: i32,
    }
    #[automatically_derived]
    impl ::core::clone::Clone for Compression {
        #[inline]
        fn clone(&self) -> Compression {
            Compression {
                algo: ::core::clone::Clone::clone(&self.algo),
                level: ::core::clone::Clone::clone(&self.level),
            }
        }
    }
    impl ::core::marker::StructuralPartialEq for Compression {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for Compression {
        #[inline]
        fn eq(&self, other: &Compression) -> bool {
            self.algo == other.algo && self.level == other.level
        }
        #[inline]
        fn ne(&self, other: &Compression) -> bool {
            self.algo != other.algo || self.level != other.level
        }
    }
    impl ::prost::Message for Compression {
        #[allow(unused_variables)]
        fn encode_raw<B>(&self, buf: &mut B)
        where
            B: ::prost::bytes::BufMut,
        {
            if self.algo != compression::CompressionAlgorithm::default() as i32 {
                ::prost::encoding::int32::encode(1u32, &self.algo, buf);
            }
            if self.level != 0i32 {
                ::prost::encoding::int32::encode(2u32, &self.level, buf);
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
            const STRUCT_NAME: &'static str = "Compression";
            match tag {
                1u32 => {
                    let mut value = &mut self.algo;
                    ::prost::encoding::int32::merge(wire_type, value, buf, ctx).map_err(
                        |mut error| {
                            error.push(STRUCT_NAME, "algo");
                            error
                        },
                    )
                }
                2u32 => {
                    let mut value = &mut self.level;
                    ::prost::encoding::int32::merge(wire_type, value, buf, ctx).map_err(
                        |mut error| {
                            error.push(STRUCT_NAME, "level");
                            error
                        },
                    )
                }
                _ => ::prost::encoding::skip_field(wire_type, tag, buf, ctx),
            }
        }
        #[inline]
        fn encoded_len(&self) -> usize {
            0 + if self.algo != compression::CompressionAlgorithm::default() as i32 {
                ::prost::encoding::int32::encoded_len(1u32, &self.algo)
            } else {
                0
            } + if self.level != 0i32 {
                ::prost::encoding::int32::encoded_len(2u32, &self.level)
            } else {
                0
            }
        }
        fn clear(&mut self) {
            self.algo = compression::CompressionAlgorithm::default() as i32;
            self.level = 0i32;
        }
    }
    impl ::core::default::Default for Compression {
        fn default() -> Self {
            Compression {
                algo: compression::CompressionAlgorithm::default() as i32,
                level: 0i32,
            }
        }
    }
    impl ::core::fmt::Debug for Compression {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            let mut builder = f.debug_struct("Compression");
            let builder = {
                let wrapper = {
                    struct ScalarWrapper<'a>(&'a i32);
                    impl<'a> ::core::fmt::Debug for ScalarWrapper<'a> {
                        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                            match compression::CompressionAlgorithm::from_i32(*self.0) {
                                None => ::core::fmt::Debug::fmt(&self.0, f),
                                Some(en) => ::core::fmt::Debug::fmt(&en, f),
                            }
                        }
                    }
                    ScalarWrapper(&self.algo)
                };
                builder.field("algo", &wrapper)
            };
            let builder = {
                let wrapper = {
                    fn ScalarWrapper<T>(v: T) -> T {
                        v
                    }
                    ScalarWrapper(&self.level)
                };
                builder.field("level", &wrapper)
            };
            builder.finish()
        }
    }
    #[allow(dead_code)]
    impl Compression {
        ///Returns the enum value of `algo`, or the default if the field is set to an invalid enum value.
        pub fn algo(&self) -> compression::CompressionAlgorithm {
            compression::CompressionAlgorithm::from_i32(self.algo)
                .unwrap_or(compression::CompressionAlgorithm::default())
        }
        ///Sets `algo` to the provided enum value.
        pub fn set_algo(&mut self, value: compression::CompressionAlgorithm) {
            self.algo = value as i32;
        }
    }
    /// Nested message and enum types in `Compression`.
    pub mod compression {
        /// CompressionAlgorithm specifies to use which algorithm to compress a block.
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
        #[automatically_derived]
        impl ::core::clone::Clone for CompressionAlgorithm {
            #[inline]
            fn clone(&self) -> CompressionAlgorithm {
                *self
            }
        }
        #[automatically_derived]
        impl ::core::marker::Copy for CompressionAlgorithm {}
        #[automatically_derived]
        impl ::core::fmt::Debug for CompressionAlgorithm {
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                match self {
                    CompressionAlgorithm::None => ::core::fmt::Formatter::write_str(f, "None"),
                    CompressionAlgorithm::Snappy => ::core::fmt::Formatter::write_str(f, "Snappy"),
                    CompressionAlgorithm::Zstd => ::core::fmt::Formatter::write_str(f, "Zstd"),
                    CompressionAlgorithm::Lz4 => ::core::fmt::Formatter::write_str(f, "Lz4"),
                }
            }
        }
        impl ::core::marker::StructuralPartialEq for CompressionAlgorithm {}
        #[automatically_derived]
        impl ::core::cmp::PartialEq for CompressionAlgorithm {
            #[inline]
            fn eq(&self, other: &CompressionAlgorithm) -> bool {
                let __self_tag = ::core::intrinsics::discriminant_value(self);
                let __arg1_tag = ::core::intrinsics::discriminant_value(other);
                __self_tag == __arg1_tag
            }
        }
        impl ::core::marker::StructuralEq for CompressionAlgorithm {}
        #[automatically_derived]
        impl ::core::cmp::Eq for CompressionAlgorithm {
            #[inline]
            #[doc(hidden)]
            #[no_coverage]
            fn assert_receiver_is_total_eq(&self) -> () {}
        }
        #[automatically_derived]
        impl ::core::hash::Hash for CompressionAlgorithm {
            fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {
                let __self_tag = ::core::intrinsics::discriminant_value(self);
                ::core::hash::Hash::hash(&__self_tag, state)
            }
        }
        #[automatically_derived]
        impl ::core::cmp::PartialOrd for CompressionAlgorithm {
            #[inline]
            fn partial_cmp(
                &self,
                other: &CompressionAlgorithm,
            ) -> ::core::option::Option<::core::cmp::Ordering> {
                let __self_tag = ::core::intrinsics::discriminant_value(self);
                let __arg1_tag = ::core::intrinsics::discriminant_value(other);
                ::core::cmp::PartialOrd::partial_cmp(&__self_tag, &__arg1_tag)
            }
        }
        #[automatically_derived]
        impl ::core::cmp::Ord for CompressionAlgorithm {
            #[inline]
            fn cmp(&self, other: &CompressionAlgorithm) -> ::core::cmp::Ordering {
                let __self_tag = ::core::intrinsics::discriminant_value(self);
                let __arg1_tag = ::core::intrinsics::discriminant_value(other);
                ::core::cmp::Ord::cmp(&__self_tag, &__arg1_tag)
            }
        }
        impl CompressionAlgorithm {
            ///Returns `true` if `value` is a variant of `CompressionAlgorithm`.
            pub fn is_valid(value: i32) -> bool {
                match value {
                    0 => true,
                    1 => true,
                    2 => true,
                    3 => true,
                    _ => false,
                }
            }
            ///Converts an `i32` to a `CompressionAlgorithm`, or `None` if `value` is not a valid variant.
            pub fn from_i32(value: i32) -> ::core::option::Option<CompressionAlgorithm> {
                match value {
                    0 => ::core::option::Option::Some(CompressionAlgorithm::None),
                    1 => ::core::option::Option::Some(CompressionAlgorithm::Snappy),
                    2 => ::core::option::Option::Some(CompressionAlgorithm::Zstd),
                    3 => ::core::option::Option::Some(CompressionAlgorithm::Lz4),
                    _ => ::core::option::Option::None,
                }
            }
        }
        impl ::core::default::Default for CompressionAlgorithm {
            fn default() -> CompressionAlgorithm {
                CompressionAlgorithm::None
            }
        }
        impl ::core::convert::From<CompressionAlgorithm> for i32 {
            fn from(value: CompressionAlgorithm) -> i32 {
                value as i32
            }
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
    pub struct Checksum {
        /// For storing type of Checksum algorithm used
        #[prost(enumeration = "checksum::ChecksumAlgorithm", tag = "1")]
        pub algo: i32,
        #[prost(uint64, tag = "2")]
        pub sum: u64,
    }
    #[automatically_derived]
    impl ::core::clone::Clone for Checksum {
        #[inline]
        fn clone(&self) -> Checksum {
            Checksum {
                algo: ::core::clone::Clone::clone(&self.algo),
                sum: ::core::clone::Clone::clone(&self.sum),
            }
        }
    }
    impl ::core::marker::StructuralPartialEq for Checksum {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for Checksum {
        #[inline]
        fn eq(&self, other: &Checksum) -> bool {
            self.algo == other.algo && self.sum == other.sum
        }
        #[inline]
        fn ne(&self, other: &Checksum) -> bool {
            self.algo != other.algo || self.sum != other.sum
        }
    }
    impl ::prost::Message for Checksum {
        #[allow(unused_variables)]
        fn encode_raw<B>(&self, buf: &mut B)
        where
            B: ::prost::bytes::BufMut,
        {
            if self.algo != checksum::ChecksumAlgorithm::default() as i32 {
                ::prost::encoding::int32::encode(1u32, &self.algo, buf);
            }
            if self.sum != 0u64 {
                ::prost::encoding::uint64::encode(2u32, &self.sum, buf);
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
            const STRUCT_NAME: &'static str = "Checksum";
            match tag {
                1u32 => {
                    let mut value = &mut self.algo;
                    ::prost::encoding::int32::merge(wire_type, value, buf, ctx).map_err(
                        |mut error| {
                            error.push(STRUCT_NAME, "algo");
                            error
                        },
                    )
                }
                2u32 => {
                    let mut value = &mut self.sum;
                    ::prost::encoding::uint64::merge(wire_type, value, buf, ctx).map_err(
                        |mut error| {
                            error.push(STRUCT_NAME, "sum");
                            error
                        },
                    )
                }
                _ => ::prost::encoding::skip_field(wire_type, tag, buf, ctx),
            }
        }
        #[inline]
        fn encoded_len(&self) -> usize {
            0 + if self.algo != checksum::ChecksumAlgorithm::default() as i32 {
                ::prost::encoding::int32::encoded_len(1u32, &self.algo)
            } else {
                0
            } + if self.sum != 0u64 {
                ::prost::encoding::uint64::encoded_len(2u32, &self.sum)
            } else {
                0
            }
        }
        fn clear(&mut self) {
            self.algo = checksum::ChecksumAlgorithm::default() as i32;
            self.sum = 0u64;
        }
    }
    impl ::core::default::Default for Checksum {
        fn default() -> Self {
            Checksum {
                algo: checksum::ChecksumAlgorithm::default() as i32,
                sum: 0u64,
            }
        }
    }
    impl ::core::fmt::Debug for Checksum {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            let mut builder = f.debug_struct("Checksum");
            let builder = {
                let wrapper = {
                    struct ScalarWrapper<'a>(&'a i32);
                    impl<'a> ::core::fmt::Debug for ScalarWrapper<'a> {
                        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                            match checksum::ChecksumAlgorithm::from_i32(*self.0) {
                                None => ::core::fmt::Debug::fmt(&self.0, f),
                                Some(en) => ::core::fmt::Debug::fmt(&en, f),
                            }
                        }
                    }
                    ScalarWrapper(&self.algo)
                };
                builder.field("algo", &wrapper)
            };
            let builder = {
                let wrapper = {
                    fn ScalarWrapper<T>(v: T) -> T {
                        v
                    }
                    ScalarWrapper(&self.sum)
                };
                builder.field("sum", &wrapper)
            };
            builder.finish()
        }
    }
    #[allow(dead_code)]
    impl Checksum {
        ///Returns the enum value of `algo`, or the default if the field is set to an invalid enum value.
        pub fn algo(&self) -> checksum::ChecksumAlgorithm {
            checksum::ChecksumAlgorithm::from_i32(self.algo)
                .unwrap_or(checksum::ChecksumAlgorithm::default())
        }
        ///Sets `algo` to the provided enum value.
        pub fn set_algo(&mut self, value: checksum::ChecksumAlgorithm) {
            self.algo = value as i32;
        }
    }
    /// Nested message and enum types in `Checksum`.
    pub mod checksum {
        #[repr(i32)]
        pub enum ChecksumAlgorithm {
            Crc32c = 0,
            XxHash64 = 1,
            SeaHash = 2,
        }
        #[automatically_derived]
        impl ::core::clone::Clone for ChecksumAlgorithm {
            #[inline]
            fn clone(&self) -> ChecksumAlgorithm {
                *self
            }
        }
        #[automatically_derived]
        impl ::core::marker::Copy for ChecksumAlgorithm {}
        #[automatically_derived]
        impl ::core::fmt::Debug for ChecksumAlgorithm {
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                match self {
                    ChecksumAlgorithm::Crc32c => ::core::fmt::Formatter::write_str(f, "Crc32c"),
                    ChecksumAlgorithm::XxHash64 => ::core::fmt::Formatter::write_str(f, "XxHash64"),
                    ChecksumAlgorithm::SeaHash => ::core::fmt::Formatter::write_str(f, "SeaHash"),
                }
            }
        }
        impl ::core::marker::StructuralPartialEq for ChecksumAlgorithm {}
        #[automatically_derived]
        impl ::core::cmp::PartialEq for ChecksumAlgorithm {
            #[inline]
            fn eq(&self, other: &ChecksumAlgorithm) -> bool {
                let __self_tag = ::core::intrinsics::discriminant_value(self);
                let __arg1_tag = ::core::intrinsics::discriminant_value(other);
                __self_tag == __arg1_tag
            }
        }
        impl ::core::marker::StructuralEq for ChecksumAlgorithm {}
        #[automatically_derived]
        impl ::core::cmp::Eq for ChecksumAlgorithm {
            #[inline]
            #[doc(hidden)]
            #[no_coverage]
            fn assert_receiver_is_total_eq(&self) -> () {}
        }
        #[automatically_derived]
        impl ::core::hash::Hash for ChecksumAlgorithm {
            fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {
                let __self_tag = ::core::intrinsics::discriminant_value(self);
                ::core::hash::Hash::hash(&__self_tag, state)
            }
        }
        #[automatically_derived]
        impl ::core::cmp::PartialOrd for ChecksumAlgorithm {
            #[inline]
            fn partial_cmp(
                &self,
                other: &ChecksumAlgorithm,
            ) -> ::core::option::Option<::core::cmp::Ordering> {
                let __self_tag = ::core::intrinsics::discriminant_value(self);
                let __arg1_tag = ::core::intrinsics::discriminant_value(other);
                ::core::cmp::PartialOrd::partial_cmp(&__self_tag, &__arg1_tag)
            }
        }
        #[automatically_derived]
        impl ::core::cmp::Ord for ChecksumAlgorithm {
            #[inline]
            fn cmp(&self, other: &ChecksumAlgorithm) -> ::core::cmp::Ordering {
                let __self_tag = ::core::intrinsics::discriminant_value(self);
                let __arg1_tag = ::core::intrinsics::discriminant_value(other);
                ::core::cmp::Ord::cmp(&__self_tag, &__arg1_tag)
            }
        }
        impl ChecksumAlgorithm {
            ///Returns `true` if `value` is a variant of `ChecksumAlgorithm`.
            pub fn is_valid(value: i32) -> bool {
                match value {
                    0 => true,
                    1 => true,
                    2 => true,
                    _ => false,
                }
            }
            ///Converts an `i32` to a `ChecksumAlgorithm`, or `None` if `value` is not a valid variant.
            pub fn from_i32(value: i32) -> ::core::option::Option<ChecksumAlgorithm> {
                match value {
                    0 => ::core::option::Option::Some(ChecksumAlgorithm::Crc32c),
                    1 => ::core::option::Option::Some(ChecksumAlgorithm::XxHash64),
                    2 => ::core::option::Option::Some(ChecksumAlgorithm::SeaHash),
                    _ => ::core::option::Option::None,
                }
            }
        }
        impl ::core::default::Default for ChecksumAlgorithm {
            fn default() -> ChecksumAlgorithm {
                ChecksumAlgorithm::Crc32c
            }
        }
        impl ::core::convert::From<ChecksumAlgorithm> for i32 {
            fn from(value: ChecksumAlgorithm) -> i32 {
                value as i32
            }
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
    pub struct DataKey {
        #[prost(uint64, tag = "1")]
        pub key_id: u64,
        #[prost(bytes = "vec", tag = "2")]
        pub data: ::prost::alloc::vec::Vec<u8>,
        #[prost(bytes = "vec", tag = "3")]
        pub iv: ::prost::alloc::vec::Vec<u8>,
        #[prost(uint64, tag = "4")]
        pub created_at: u64,
    }
    #[automatically_derived]
    impl ::core::clone::Clone for DataKey {
        #[inline]
        fn clone(&self) -> DataKey {
            DataKey {
                key_id: ::core::clone::Clone::clone(&self.key_id),
                data: ::core::clone::Clone::clone(&self.data),
                iv: ::core::clone::Clone::clone(&self.iv),
                created_at: ::core::clone::Clone::clone(&self.created_at),
            }
        }
    }
    impl ::core::marker::StructuralPartialEq for DataKey {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for DataKey {
        #[inline]
        fn eq(&self, other: &DataKey) -> bool {
            self.key_id == other.key_id
                && self.data == other.data
                && self.iv == other.iv
                && self.created_at == other.created_at
        }
        #[inline]
        fn ne(&self, other: &DataKey) -> bool {
            self.key_id != other.key_id
                || self.data != other.data
                || self.iv != other.iv
                || self.created_at != other.created_at
        }
    }
    impl ::prost::Message for DataKey {
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
            const STRUCT_NAME: &'static str = "DataKey";
            match tag {
                1u32 => {
                    let mut value = &mut self.key_id;
                    ::prost::encoding::uint64::merge(wire_type, value, buf, ctx).map_err(
                        |mut error| {
                            error.push(STRUCT_NAME, "key_id");
                            error
                        },
                    )
                }
                2u32 => {
                    let mut value = &mut self.data;
                    ::prost::encoding::bytes::merge(wire_type, value, buf, ctx).map_err(
                        |mut error| {
                            error.push(STRUCT_NAME, "data");
                            error
                        },
                    )
                }
                3u32 => {
                    let mut value = &mut self.iv;
                    ::prost::encoding::bytes::merge(wire_type, value, buf, ctx).map_err(
                        |mut error| {
                            error.push(STRUCT_NAME, "iv");
                            error
                        },
                    )
                }
                4u32 => {
                    let mut value = &mut self.created_at;
                    ::prost::encoding::uint64::merge(wire_type, value, buf, ctx).map_err(
                        |mut error| {
                            error.push(STRUCT_NAME, "created_at");
                            error
                        },
                    )
                }
                _ => ::prost::encoding::skip_field(wire_type, tag, buf, ctx),
            }
        }
        #[inline]
        fn encoded_len(&self) -> usize {
            0 + if self.key_id != 0u64 {
                ::prost::encoding::uint64::encoded_len(1u32, &self.key_id)
            } else {
                0
            } + if self.data != b"" as &[u8] {
                ::prost::encoding::bytes::encoded_len(2u32, &self.data)
            } else {
                0
            } + if self.iv != b"" as &[u8] {
                ::prost::encoding::bytes::encoded_len(3u32, &self.iv)
            } else {
                0
            } + if self.created_at != 0u64 {
                ::prost::encoding::uint64::encoded_len(4u32, &self.created_at)
            } else {
                0
            }
        }
        fn clear(&mut self) {
            self.key_id = 0u64;
            self.data.clear();
            self.iv.clear();
            self.created_at = 0u64;
        }
    }
    impl ::core::default::Default for DataKey {
        fn default() -> Self {
            DataKey {
                key_id: 0u64,
                data: ::core::default::Default::default(),
                iv: ::core::default::Default::default(),
                created_at: 0u64,
            }
        }
    }
    impl ::core::fmt::Debug for DataKey {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            let mut builder = f.debug_struct("DataKey");
            let builder = {
                let wrapper = {
                    fn ScalarWrapper<T>(v: T) -> T {
                        v
                    }
                    ScalarWrapper(&self.key_id)
                };
                builder.field("key_id", &wrapper)
            };
            let builder = {
                let wrapper = {
                    fn ScalarWrapper<T>(v: T) -> T {
                        v
                    }
                    ScalarWrapper(&self.data)
                };
                builder.field("data", &wrapper)
            };
            let builder = {
                let wrapper = {
                    fn ScalarWrapper<T>(v: T) -> T {
                        v
                    }
                    ScalarWrapper(&self.iv)
                };
                builder.field("iv", &wrapper)
            };
            let builder = {
                let wrapper = {
                    fn ScalarWrapper<T>(v: T) -> T {
                        v
                    }
                    ScalarWrapper(&self.created_at)
                };
                builder.field("created_at", &wrapper)
            };
            builder.finish()
        }
    }
    pub struct Match {
        #[prost(bytes = "vec", tag = "1")]
        pub prefix: ::prost::alloc::vec::Vec<u8>,
        /// Comma separated with dash to represent ranges "1, 2-3, 4-7, 9"
        #[prost(string, tag = "2")]
        pub ignore_bytes: ::prost::alloc::string::String,
    }
    #[automatically_derived]
    impl ::core::clone::Clone for Match {
        #[inline]
        fn clone(&self) -> Match {
            Match {
                prefix: ::core::clone::Clone::clone(&self.prefix),
                ignore_bytes: ::core::clone::Clone::clone(&self.ignore_bytes),
            }
        }
    }
    impl ::core::marker::StructuralPartialEq for Match {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for Match {
        #[inline]
        fn eq(&self, other: &Match) -> bool {
            self.prefix == other.prefix && self.ignore_bytes == other.ignore_bytes
        }
        #[inline]
        fn ne(&self, other: &Match) -> bool {
            self.prefix != other.prefix || self.ignore_bytes != other.ignore_bytes
        }
    }
    impl ::prost::Message for Match {
        #[allow(unused_variables)]
        fn encode_raw<B>(&self, buf: &mut B)
        where
            B: ::prost::bytes::BufMut,
        {
            if self.prefix != b"" as &[u8] {
                ::prost::encoding::bytes::encode(1u32, &self.prefix, buf);
            }
            if self.ignore_bytes != "" {
                ::prost::encoding::string::encode(2u32, &self.ignore_bytes, buf);
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
            const STRUCT_NAME: &'static str = "Match";
            match tag {
                1u32 => {
                    let mut value = &mut self.prefix;
                    ::prost::encoding::bytes::merge(wire_type, value, buf, ctx).map_err(
                        |mut error| {
                            error.push(STRUCT_NAME, "prefix");
                            error
                        },
                    )
                }
                2u32 => {
                    let mut value = &mut self.ignore_bytes;
                    ::prost::encoding::string::merge(wire_type, value, buf, ctx).map_err(
                        |mut error| {
                            error.push(STRUCT_NAME, "ignore_bytes");
                            error
                        },
                    )
                }
                _ => ::prost::encoding::skip_field(wire_type, tag, buf, ctx),
            }
        }
        #[inline]
        fn encoded_len(&self) -> usize {
            0 + if self.prefix != b"" as &[u8] {
                ::prost::encoding::bytes::encoded_len(1u32, &self.prefix)
            } else {
                0
            } + if self.ignore_bytes != "" {
                ::prost::encoding::string::encoded_len(2u32, &self.ignore_bytes)
            } else {
                0
            }
        }
        fn clear(&mut self) {
            self.prefix.clear();
            self.ignore_bytes.clear();
        }
    }
    impl ::core::default::Default for Match {
        fn default() -> Self {
            Match {
                prefix: ::core::default::Default::default(),
                ignore_bytes: ::prost::alloc::string::String::new(),
            }
        }
    }
    impl ::core::fmt::Debug for Match {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            let mut builder = f.debug_struct("Match");
            let builder = {
                let wrapper = {
                    fn ScalarWrapper<T>(v: T) -> T {
                        v
                    }
                    ScalarWrapper(&self.prefix)
                };
                builder.field("prefix", &wrapper)
            };
            let builder = {
                let wrapper = {
                    fn ScalarWrapper<T>(v: T) -> T {
                        v
                    }
                    ScalarWrapper(&self.ignore_bytes)
                };
                builder.field("ignore_bytes", &wrapper)
            };
            builder.finish()
        }
    }
    pub struct BlockOffset {
        #[prost(bytes = "vec", tag = "1")]
        pub key: ::prost::alloc::vec::Vec<u8>,
        #[prost(uint32, tag = "2")]
        pub offset: u32,
        #[prost(uint32, tag = "3")]
        pub len: u32,
    }
    #[automatically_derived]
    impl ::core::clone::Clone for BlockOffset {
        #[inline]
        fn clone(&self) -> BlockOffset {
            BlockOffset {
                key: ::core::clone::Clone::clone(&self.key),
                offset: ::core::clone::Clone::clone(&self.offset),
                len: ::core::clone::Clone::clone(&self.len),
            }
        }
    }
    impl ::core::marker::StructuralPartialEq for BlockOffset {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for BlockOffset {
        #[inline]
        fn eq(&self, other: &BlockOffset) -> bool {
            self.key == other.key && self.offset == other.offset && self.len == other.len
        }
        #[inline]
        fn ne(&self, other: &BlockOffset) -> bool {
            self.key != other.key || self.offset != other.offset || self.len != other.len
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
            const STRUCT_NAME: &'static str = "BlockOffset";
            match tag {
                1u32 => {
                    let mut value = &mut self.key;
                    ::prost::encoding::bytes::merge(wire_type, value, buf, ctx).map_err(
                        |mut error| {
                            error.push(STRUCT_NAME, "key");
                            error
                        },
                    )
                }
                2u32 => {
                    let mut value = &mut self.offset;
                    ::prost::encoding::uint32::merge(wire_type, value, buf, ctx).map_err(
                        |mut error| {
                            error.push(STRUCT_NAME, "offset");
                            error
                        },
                    )
                }
                3u32 => {
                    let mut value = &mut self.len;
                    ::prost::encoding::uint32::merge(wire_type, value, buf, ctx).map_err(
                        |mut error| {
                            error.push(STRUCT_NAME, "len");
                            error
                        },
                    )
                }
                _ => ::prost::encoding::skip_field(wire_type, tag, buf, ctx),
            }
        }
        #[inline]
        fn encoded_len(&self) -> usize {
            0 + if self.key != b"" as &[u8] {
                ::prost::encoding::bytes::encoded_len(1u32, &self.key)
            } else {
                0
            } + if self.offset != 0u32 {
                ::prost::encoding::uint32::encoded_len(2u32, &self.offset)
            } else {
                0
            } + if self.len != 0u32 {
                ::prost::encoding::uint32::encoded_len(3u32, &self.len)
            } else {
                0
            }
        }
        fn clear(&mut self) {
            self.key.clear();
            self.offset = 0u32;
            self.len = 0u32;
        }
    }
    impl ::core::default::Default for BlockOffset {
        fn default() -> Self {
            BlockOffset {
                key: ::core::default::Default::default(),
                offset: 0u32,
                len: 0u32,
            }
        }
    }
    impl ::core::fmt::Debug for BlockOffset {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            let mut builder = f.debug_struct("BlockOffset");
            let builder = {
                let wrapper = {
                    fn ScalarWrapper<T>(v: T) -> T {
                        v
                    }
                    ScalarWrapper(&self.key)
                };
                builder.field("key", &wrapper)
            };
            let builder = {
                let wrapper = {
                    fn ScalarWrapper<T>(v: T) -> T {
                        v
                    }
                    ScalarWrapper(&self.offset)
                };
                builder.field("offset", &wrapper)
            };
            let builder = {
                let wrapper = {
                    fn ScalarWrapper<T>(v: T) -> T {
                        v
                    }
                    ScalarWrapper(&self.len)
                };
                builder.field("len", &wrapper)
            };
            builder.finish()
        }
    }
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
    #[automatically_derived]
    impl ::core::clone::Clone for TableIndex {
        #[inline]
        fn clone(&self) -> TableIndex {
            TableIndex {
                offsets: ::core::clone::Clone::clone(&self.offsets),
                bloom_filter: ::core::clone::Clone::clone(&self.bloom_filter),
                estimated_size: ::core::clone::Clone::clone(&self.estimated_size),
                max_version: ::core::clone::Clone::clone(&self.max_version),
                key_count: ::core::clone::Clone::clone(&self.key_count),
                uncompressed_size: ::core::clone::Clone::clone(&self.uncompressed_size),
                stale_data_size: ::core::clone::Clone::clone(&self.stale_data_size),
            }
        }
    }
    impl ::core::marker::StructuralPartialEq for TableIndex {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for TableIndex {
        #[inline]
        fn eq(&self, other: &TableIndex) -> bool {
            self.offsets == other.offsets
                && self.bloom_filter == other.bloom_filter
                && self.estimated_size == other.estimated_size
                && self.max_version == other.max_version
                && self.key_count == other.key_count
                && self.uncompressed_size == other.uncompressed_size
                && self.stale_data_size == other.stale_data_size
        }
        #[inline]
        fn ne(&self, other: &TableIndex) -> bool {
            self.offsets != other.offsets
                || self.bloom_filter != other.bloom_filter
                || self.estimated_size != other.estimated_size
                || self.max_version != other.max_version
                || self.key_count != other.key_count
                || self.uncompressed_size != other.uncompressed_size
                || self.stale_data_size != other.stale_data_size
        }
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
            const STRUCT_NAME: &'static str = "TableIndex";
            match tag {
                1u32 => {
                    let mut value = &mut self.offsets;
                    ::prost::encoding::message::merge_repeated(wire_type, value, buf, ctx).map_err(
                        |mut error| {
                            error.push(STRUCT_NAME, "offsets");
                            error
                        },
                    )
                }
                2u32 => {
                    let mut value = &mut self.bloom_filter;
                    ::prost::encoding::bytes::merge(wire_type, value, buf, ctx).map_err(
                        |mut error| {
                            error.push(STRUCT_NAME, "bloom_filter");
                            error
                        },
                    )
                }
                3u32 => {
                    let mut value = &mut self.estimated_size;
                    ::prost::encoding::uint32::merge(wire_type, value, buf, ctx).map_err(
                        |mut error| {
                            error.push(STRUCT_NAME, "estimated_size");
                            error
                        },
                    )
                }
                4u32 => {
                    let mut value = &mut self.max_version;
                    ::prost::encoding::uint64::merge(wire_type, value, buf, ctx).map_err(
                        |mut error| {
                            error.push(STRUCT_NAME, "max_version");
                            error
                        },
                    )
                }
                5u32 => {
                    let mut value = &mut self.key_count;
                    ::prost::encoding::uint32::merge(wire_type, value, buf, ctx).map_err(
                        |mut error| {
                            error.push(STRUCT_NAME, "key_count");
                            error
                        },
                    )
                }
                6u32 => {
                    let mut value = &mut self.uncompressed_size;
                    ::prost::encoding::uint32::merge(wire_type, value, buf, ctx).map_err(
                        |mut error| {
                            error.push(STRUCT_NAME, "uncompressed_size");
                            error
                        },
                    )
                }
                7u32 => {
                    let mut value = &mut self.stale_data_size;
                    ::prost::encoding::uint32::merge(wire_type, value, buf, ctx).map_err(
                        |mut error| {
                            error.push(STRUCT_NAME, "stale_data_size");
                            error
                        },
                    )
                }
                _ => ::prost::encoding::skip_field(wire_type, tag, buf, ctx),
            }
        }
        #[inline]
        fn encoded_len(&self) -> usize {
            0 + ::prost::encoding::message::encoded_len_repeated(1u32, &self.offsets)
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
    impl ::core::default::Default for TableIndex {
        fn default() -> Self {
            TableIndex {
                offsets: ::core::default::Default::default(),
                bloom_filter: ::core::default::Default::default(),
                estimated_size: 0u32,
                max_version: 0u64,
                key_count: 0u32,
                uncompressed_size: 0u32,
                stale_data_size: 0u32,
            }
        }
    }
    impl ::core::fmt::Debug for TableIndex {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            let mut builder = f.debug_struct("TableIndex");
            let builder = {
                let wrapper = &self.offsets;
                builder.field("offsets", &wrapper)
            };
            let builder = {
                let wrapper = {
                    fn ScalarWrapper<T>(v: T) -> T {
                        v
                    }
                    ScalarWrapper(&self.bloom_filter)
                };
                builder.field("bloom_filter", &wrapper)
            };
            let builder = {
                let wrapper = {
                    fn ScalarWrapper<T>(v: T) -> T {
                        v
                    }
                    ScalarWrapper(&self.estimated_size)
                };
                builder.field("estimated_size", &wrapper)
            };
            let builder = {
                let wrapper = {
                    fn ScalarWrapper<T>(v: T) -> T {
                        v
                    }
                    ScalarWrapper(&self.max_version)
                };
                builder.field("max_version", &wrapper)
            };
            let builder = {
                let wrapper = {
                    fn ScalarWrapper<T>(v: T) -> T {
                        v
                    }
                    ScalarWrapper(&self.key_count)
                };
                builder.field("key_count", &wrapper)
            };
            let builder = {
                let wrapper = {
                    fn ScalarWrapper<T>(v: T) -> T {
                        v
                    }
                    ScalarWrapper(&self.uncompressed_size)
                };
                builder.field("uncompressed_size", &wrapper)
            };
            let builder = {
                let wrapper = {
                    fn ScalarWrapper<T>(v: T) -> T {
                        v
                    }
                    ScalarWrapper(&self.stale_data_size)
                };
                builder.field("stale_data_size", &wrapper)
            };
            builder.finish()
        }
    }
    #[repr(i32)]
    pub enum EncryptionAlgorithm {
        None = 0,
        Aes = 1,
    }
    #[automatically_derived]
    impl ::core::clone::Clone for EncryptionAlgorithm {
        #[inline]
        fn clone(&self) -> EncryptionAlgorithm {
            *self
        }
    }
    #[automatically_derived]
    impl ::core::marker::Copy for EncryptionAlgorithm {}
    #[automatically_derived]
    impl ::core::fmt::Debug for EncryptionAlgorithm {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match self {
                EncryptionAlgorithm::None => ::core::fmt::Formatter::write_str(f, "None"),
                EncryptionAlgorithm::Aes => ::core::fmt::Formatter::write_str(f, "Aes"),
            }
        }
    }
    impl ::core::marker::StructuralPartialEq for EncryptionAlgorithm {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for EncryptionAlgorithm {
        #[inline]
        fn eq(&self, other: &EncryptionAlgorithm) -> bool {
            let __self_tag = ::core::intrinsics::discriminant_value(self);
            let __arg1_tag = ::core::intrinsics::discriminant_value(other);
            __self_tag == __arg1_tag
        }
    }
    impl ::core::marker::StructuralEq for EncryptionAlgorithm {}
    #[automatically_derived]
    impl ::core::cmp::Eq for EncryptionAlgorithm {
        #[inline]
        #[doc(hidden)]
        #[no_coverage]
        fn assert_receiver_is_total_eq(&self) -> () {}
    }
    #[automatically_derived]
    impl ::core::hash::Hash for EncryptionAlgorithm {
        fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) -> () {
            let __self_tag = ::core::intrinsics::discriminant_value(self);
            ::core::hash::Hash::hash(&__self_tag, state)
        }
    }
    #[automatically_derived]
    impl ::core::cmp::PartialOrd for EncryptionAlgorithm {
        #[inline]
        fn partial_cmp(
            &self,
            other: &EncryptionAlgorithm,
        ) -> ::core::option::Option<::core::cmp::Ordering> {
            let __self_tag = ::core::intrinsics::discriminant_value(self);
            let __arg1_tag = ::core::intrinsics::discriminant_value(other);
            ::core::cmp::PartialOrd::partial_cmp(&__self_tag, &__arg1_tag)
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Ord for EncryptionAlgorithm {
        #[inline]
        fn cmp(&self, other: &EncryptionAlgorithm) -> ::core::cmp::Ordering {
            let __self_tag = ::core::intrinsics::discriminant_value(self);
            let __arg1_tag = ::core::intrinsics::discriminant_value(other);
            ::core::cmp::Ord::cmp(&__self_tag, &__arg1_tag)
        }
    }
    impl EncryptionAlgorithm {
        ///Returns `true` if `value` is a variant of `EncryptionAlgorithm`.
        pub fn is_valid(value: i32) -> bool {
            match value {
                0 => true,
                1 => true,
                _ => false,
            }
        }
        ///Converts an `i32` to a `EncryptionAlgorithm`, or `None` if `value` is not a valid variant.
        pub fn from_i32(value: i32) -> ::core::option::Option<EncryptionAlgorithm> {
            match value {
                0 => ::core::option::Option::Some(EncryptionAlgorithm::None),
                1 => ::core::option::Option::Some(EncryptionAlgorithm::Aes),
                _ => ::core::option::Option::None,
            }
        }
    }
    impl ::core::default::Default for EncryptionAlgorithm {
        fn default() -> EncryptionAlgorithm {
            EncryptionAlgorithm::None
        }
    }
    impl ::core::convert::From<EncryptionAlgorithm> for i32 {
        fn from(value: EncryptionAlgorithm) -> i32 {
            value as i32
        }
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
}
