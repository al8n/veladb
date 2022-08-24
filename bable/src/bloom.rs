use core::f64::consts::LN_2;

use vpb::kvstructs::bytes::{BufMut, Bytes, BytesMut};

const SEED: u32 = 0xbc9f1d34;
const M: u32 = 0xc6a4a793;

/// hash implements a hashing algorithm similar to the Murmur hash.
pub fn hash(b: &[u8]) -> u32 {
    let mut h = SEED ^ M.wrapping_mul(b.len() as u32);

    let mut c = b;
    while c.len() >= 4 {
        h = h.wrapping_add(
            (c[0] as u32)
                | (c[1] as u32).wrapping_shl(8)
                | (c[2] as u32).wrapping_shl(16)
                | (c[3] as u32).wrapping_shl(24),
        );
        h = h.wrapping_mul(M);
        h ^= h.wrapping_shr(16);
        c = &c[4..];
    }

    match c.len() {
        3 => {
            h = h.wrapping_add((c[2] as u32) << 16);
            h = h.wrapping_add((c[1] as u32) << 8);
            h = h.wrapping_add(c[0] as u32);
            h = h.wrapping_mul(M);
            h ^= h.wrapping_shr(24);
        }
        2 => {
            h = h.wrapping_add((c[1] as u32) << 8);
            h = h.wrapping_add(c[0] as u32);
            h = h.wrapping_mul(M);
            h ^= h.wrapping_shr(24);
        }
        1 => {
            h += c[0] as u32;
            h = h.wrapping_mul(M);
            h ^= h.wrapping_shr(24);
        }
        _ => {}
    }
    h
}

/// Returns the bits per key required by bloomfilter based on
/// the false positive rate.
pub fn bloom_bits_per_key(num_entries: usize, fp: f64) -> usize {
    let fne = num_entries as f64;
    let size = -1f64 * fne * fp.ln() / LN_2.powi(2);
    let locs = LN_2.ceil() * size / fne;
    locs as usize
}

/// Filter is an encoded set of Vec<u8> keys.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
#[repr(transparent)]
pub struct Filter(Bytes);

impl core::ops::Deref for Filter {
    type Target = Bytes;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Default for Filter {
    fn default() -> Self {
        Self(Bytes::new())
    }
}

impl AsRef<[u8]> for Filter {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl Filter {
    pub fn new(keys: &[u32], bits_per_key: usize) -> Self {
        let k = ((0.69 * bits_per_key as f64) as u32).max(1).min(30);

        // For small len(keys), we can see a very high false positive rate. Fix it
        // by enforcing a minimum bloom filter length to 64.
        let mut nbs = (keys.len() * bits_per_key).max(64);

        let nbytes = (nbs + 7) / 8;
        nbs = nbytes * 8;

        let mut filter = BytesMut::with_capacity(nbytes + 1);
        filter.resize(nbytes, 0);

        for h in keys {
            let mut h = *h;
            let delta = h >> 17 | h << 15;
            (0..k).for_each(|_| {
                let bit_pos = h % (nbs as u32);
                filter[(bit_pos / 8) as usize] |= 1 << (bit_pos % 8);
                h = h.wrapping_add(delta);
            });
        }

        filter.put_u8(k as u8);
        Filter(filter.freeze())
    }

    #[inline]
    pub const fn len(&self) -> usize {
        self.0.len()
    }

    #[inline]
    pub const fn cap(&self) -> usize {
        self.0.len()
    }

    #[inline]
    pub fn into_vec(self) -> Vec<u8> {
        self.0.to_vec()
    }
}

pub trait MayContain {
    /// may_contain returns whether the filter may contain given key. False positives
    /// are possible, where it returns true for keys not in the original set.
    #[inline]
    fn may_contain(&self, mut h: u32) -> bool
    where
        Self: AsRef<[u8]>,
    {
        let slice = self.as_ref();
        let len = slice.len();
        if len < 2 {
            false
        } else {
            let k = slice[len - 1];
            if k > 30 {
                true
            } else {
                let nbs = (8 * (len - 1)) as u32;
                let delta = h >> 17 | h << 15;
                for _ in 0..k {
                    let bit_pos = h % nbs;
                    if slice[(bit_pos / 8) as usize] & (1 << (bit_pos % 8)) == 0 {
                        return false;
                    }
                    h = h.wrapping_add(delta);
                }
                true
            }
        }
    }

    #[inline]
    fn may_contain_key(&self, k: &[u8]) -> bool
    where
        Self: AsRef<[u8]>,
    {
        self.may_contain(hash(k))
    }
}

impl<T> MayContain for T {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    impl ToString for Filter {
        fn to_string(&self) -> String {
            let mut s = vec!["."; 8 * self.len()];
            for (i, x) in self.0.iter().enumerate() {
                for j in 0..8 {
                    if *x & (1 << j) != 0 {
                        s[8 * i + j] = "1";
                    }
                }
            }
            s.join("")
        }
    }

    #[test]
    fn test_small_filter() {
        let mut hashes = Vec::new();
        for word in &["hello".as_bytes(), "world".as_bytes()] {
            hashes.push(hash(word));
        }

        let f = Filter::new(hashes.as_slice(), 10);
        assert_eq!(
            "1...1.........1.........1.....1...1...1.....1.........1.....1....11.....",
            f.to_string()
        );

        let mut m = HashMap::new();
        m.insert("hello", true);
        m.insert("world", true);
        m.insert("x", false);
        m.insert("foo", false);

        for (k, v) in m {
            assert_eq!(f.may_contain_key(k.as_bytes()), v);
        }
    }

    #[test]
    fn test_filter() {
        fn nxt_len(x: usize) -> usize {
            if x < 10 {
                return x + 1;
            }

            if x < 100 {
                return x + 10;
            }

            if x < 1000 {
                return x + 100;
            }
            x + 1000
        }

        fn le32(i: usize) -> Vec<u8> {
            let mut b = Vec::new();
            let i = i as u32;
            b.push(i as u8);
            b.push((i >> 8) as u8);
            b.push((i >> 16) as u8);
            b.push((i >> 24) as u8);
            b
        }

        let (mut num_mediocre_filters, mut num_good_filters) = (0, 0);
        'outer: loop {
            let mut len = 1;
            while len <= 10_000 {
                let keys = (0..len).map(le32).collect::<Vec<_>>();

                let hashes = keys
                    .iter()
                    .map(|key| hash(key.as_slice()))
                    .collect::<Vec<_>>();

                let f = Filter::new(hashes.as_slice(), 10);
                if f.len() > (len * 10 / 8) + 40 {
                    eprintln!("len={}: f.len()={} is too large", len, f.len());
                    continue;
                }

                // All added keys must match.
                for k in &keys {
                    if !f.may_contain_key(k.as_slice()) {
                        eprintln!("len={}: did not contain key {:?}", len, k);
                        continue 'outer;
                    }
                }

                // check false positive rate.
                let mut num_false_positive = 0;
                for i in 0..10000 {
                    if f.may_contain_key(le32(1e9 as usize + i).as_slice()) {
                        num_false_positive += 1;
                    }
                }

                if num_false_positive > (0.02 * 10_000f64) as usize {
                    eprintln!(
                        "len={}: {} false positives in 10000",
                        len, num_false_positive
                    );
                    continue;
                }

                if num_false_positive > (0.0125 * 10_000f64) as usize {
                    num_mediocre_filters += 1;
                } else {
                    num_good_filters += 1;
                }

                len = nxt_len(len);
            }
            break;
        }

        assert!(
            num_mediocre_filters <= num_good_filters / 5,
            "{} mediocre filters but only {} good filters",
            num_mediocre_filters,
            num_good_filters
        );
    }

    #[test]
    fn test_hash() {
        // The magic want numbers come from running the C++ leveldb code in hash.cc.
        let mut m = HashMap::new();
        m.insert("", 3164544308);
        m.insert("r", 1499315714);
        m.insert("ru", 2735451247);
        m.insert("rus", 1743522570);
        m.insert("rust", 3428276233);
        m.insert("rusta", 2505947263);
        m.insert("rustac", 1658958784);
        m.insert("rustace", 1755101662);
        m.insert("rustacea", 2293694788);
        m.insert("rustacean", 2757417412);
        m.insert("I had a dream it would end this way.", 0xe14a9db9);
        for (k, v) in m {
            assert_eq!(hash(k.as_bytes()), v, "key: {}", k);
        }
    }
}
