use super::*;
use crate::TableOptions;
use rand::{thread_rng, Rng};
use std::sync::Arc;
use std::thread::spawn;
use vpb::kvstructs::{KeyExt, Value, ValueExt};
use zallocator::pool::AllocatorPool;

fn key(prefix: &str, i: isize) -> String {
    format!("{}{:04}", prefix, i)
}

pub(crate) fn get_test_table_options() -> TableOptions {
    use crate::MAX_ALLOCATOR_INITIAL_SIZE;

    TableOptions::default_with_pool(AllocatorPool::new(
        ((2 << 20) * 2).min(MAX_ALLOCATOR_INITIAL_SIZE),
    ))
}

#[derive(PartialEq, Eq)]
pub(crate) struct KV {
    index: usize,
    data: Vec<Vec<u8>>,
}

impl PartialOrd<Self> for KV {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for KV {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.data[0].cmp(&other.data[0])
    }
}

fn build_test_table(prefix: &str, n: usize, mut opts: TableOptions) -> Result<Table> {
    if opts.block_size().eq(&0) {
        opts = opts.set_block_size(4 * 1024);
    }
    assert!(n <= 10_000);
    let key_values = (0..n)
        .map(|i| KV {
            index: i,
            data: vec![
                key(prefix, i as isize).as_bytes().to_vec(),
                format!("{}", i).as_bytes().to_vec(),
            ],
        })
        .collect::<Vec<_>>();

    build_table(key_values, opts)
}

fn build_table(mut key_values: Vec<KV>, opts: TableOptions) -> Result<Table> {
    let mut b = Builder::new(RefCounter::new(opts)).unwrap();
    let mut rng = thread_rng();
    let mut filename = std::env::temp_dir();
    filename.push(rng.gen::<u32>().to_string());
    filename.set_extension("sst");
    key_values.sort();
    key_values.into_iter().for_each(|kv| {
        assert_eq!(kv.data.len(), 2);
        b.insert(
            Key::from(kv.data[0].clone()).with_timestamp(0),
            Value::with_all_fields('A' as u8, 0, 0, 0, Bytes::from(kv.data[1].clone())),
            0,
        );
    });
    Ok(Table::create_table(filename, b).unwrap())
}

#[test]
fn test_table_big_value() {
    fn val(i: usize) -> Bytes {
        // Return 1MB value which is > math.MaxUint16.
        Bytes::from(format!("{:01048576}", i))
        // Bytes::from(format!("{}", i))
    }

    let mut rng = thread_rng();

    let n = 100usize;
    let opts = get_test_table_options()
        .set_table_size((n as u64) << 20)
        .set_block_size(4 * 1024)
        .set_bloom_ratio(0.01);
    let mut builder = Builder::new(RefCounter::new(opts)).unwrap();
    for i in 0..n {
        let k = Key::from(key("", i as isize)).with_timestamp((i + 1) as u64);
        let v = Value::from(val(i));
        builder.insert(k, v, 0)
    }

    let mut filename = std::env::temp_dir();

    filename.push(rng.gen::<u32>().to_string());
    filename.set_extension("sst");

    let tbl = Table::create_table(filename, builder).unwrap();

    let mut iter = tbl.iter(0);
    iter.next();
    let mut cnt = 0;
    while iter.valid() {
        assert_eq!(key("", cnt).as_bytes(), iter.key().unwrap().parse_key());
        assert_eq!(
            val(cnt as usize).as_ref(),
            iter.val().unwrap().parse_value()
        );
        cnt += 1;
        iter.next();
    }

    assert!(!iter.valid());
    assert_eq!(n as u64, cnt as u64);
    assert_eq!(n as u64, tbl.max_version());
}

// #[test]
// fn test_table_checksum() {
//     let mut rb: [u8; 100] = [0; 100];
//     let mut rng = thread_rng();
//     rng.fill_bytes(&mut rb);
//     let mut opts = get_test_table_options();
//     opts.checksum_mode = ChecksumVerificationMode::OnTableAndBlockRead;
//     let tbl = build_test_table("k", 5, opts.clone()).unwrap();
//     // Write random bytes at random location.
//     let start = rng.gen_range(1..tbl.mmap.len() - rb.len());
//     tbl.mmap.write(rb.as_ref(), start);
//     let err = RawTable::open_table(tbl.mmap, None, Arc::new(opts)).err().unwrap();
//     assert_eq!(err.kind(), ErrorKind::ChecksumMismatch);
// }

#[test]
fn test_does_not_have_race() {
    let opts = get_test_table_options();
    let table = build_test_table("key", 10_000, opts).unwrap();
    let tbl = Arc::new(table);
    let wg = Arc::new(());
    for _ in 0..5 {
        let twg = wg.clone();
        let ttbl = tbl.clone();
        spawn(move || {
            assert!(ttbl.contains_hash(1237882));
            let _ = twg;
        });
    }
    while Arc::strong_count(&wg) > 1 {}
}

#[test]
fn test_max_version() {
    let opts = get_test_table_options();
    let mut b = Builder::new(RefCounter::new(opts)).unwrap();
    let mut rng = thread_rng();
    let mut filename = std::env::temp_dir();

    filename.push(rng.gen::<u32>().to_string());
    filename.set_extension("sst");

    let n = 1000;
    for i in 0..n {
        b.insert(
            Key::from(format!("foo:{}", i)).with_timestamp(i + 1),
            Value::default(),
            0,
        );
    }

    let tbl = Table::create_table(filename, b).unwrap();
    assert_eq!(n, tbl.max_version());
}

#[test]
fn test_table_iterator() {
    fn runner(n: usize) {
        let opts = get_test_table_options();
        let table = build_test_table("key", n, opts).unwrap();
        let mut iter = table.iter(0);
        let mut cnt = 0;
        iter.next();
        for _ in 0..iter.count() {
            let v = iter.val().unwrap();
            assert_eq!(format!("{}", cnt).as_bytes(), v.parse_value());
            cnt += 1;
            iter.next();
        }
        assert_eq!(cnt as usize, n);
    }
    for n in [99, 100, 101, 199, 200, 250, 9999, 10000] {
        runner(n);
    }
}

#[test]
fn test_seek_to_first() {
    fn runner(n: usize) {
        let opts = get_test_table_options();
        let table = build_test_table("key", n, opts).unwrap();
        let mut iter = table.iter(0);
        iter.seek_to_first();
        assert!(iter.valid());
        let v = iter.val().unwrap();
        assert_eq!(v.parse_value(), "0".to_string().as_bytes());
        assert_eq!(v.get_meta(), 'A' as u8);
    }

    for n in [99, 100, 101, 199, 200, 250, 9999, 10000] {
        runner(n);
    }
}

#[test]
fn test_seek_to_last() {
    fn runner(n: usize) {
        let opts = get_test_table_options();
        let table = build_test_table("key", n, opts).unwrap();
        let mut iter = table.iter(0);
        iter.seek_to_last();
        assert!(iter.valid());
        let v = iter.val().unwrap();
        assert_eq!(v.parse_value(), format!("{}", n - 1).as_bytes());
        assert_eq!(v.get_meta(), 'A' as u8);
        iter.prev();
        assert!(iter.valid());
        let v = iter.val().unwrap();
        assert_eq!(v.parse_value(), format!("{}", n - 2).as_bytes());
        assert_eq!(v.get_meta(), 'A' as u8);
    }

    for n in [99, 100, 101, 199, 200, 250, 9999, 10000] {
        runner(n);
    }
}

struct TestData {
    input: Vec<u8>,
    valid: bool,
    output: Vec<u8>,
}

#[test]
fn test_seek_in() {
    let opts = get_test_table_options();
    let table = build_test_table("k", 10_000, opts).unwrap();
    let mut iter = table.iter(0);

    let data = vec![
        TestData {
            input: "abc".to_string().as_bytes().to_vec(),
            valid: true,
            output: "k0000".to_string().as_bytes().to_vec(),
        },
        TestData {
            input: "k0100".to_string().as_bytes().to_vec(),
            valid: true,
            output: "k0100".to_string().as_bytes().to_vec(),
        },
        TestData {
            input: "k0100b".to_string().as_bytes().to_vec(),
            valid: true,
            output: "k0101".to_string().as_bytes().to_vec(),
        }, // Test case where we jump to next block.
        TestData {
            input: "k1234".to_string().as_bytes().to_vec(),
            valid: true,
            output: "k1234".to_string().as_bytes().to_vec(),
        },
        TestData {
            input: "k1234b".to_string().as_bytes().to_vec(),
            valid: true,
            output: "k1235".to_string().as_bytes().to_vec(),
        },
        TestData {
            input: "k9999".to_string().as_bytes().to_vec(),
            valid: true,
            output: "k9999".to_string().as_bytes().to_vec(),
        },
        TestData {
            input: "z".to_string().as_bytes().to_vec(),
            valid: false,
            output: "".to_string().as_bytes().to_vec(),
        },
    ];

    for tt in data {
        iter.seek(Key::from(tt.input.clone()).with_timestamp(0).as_slice());
        if !tt.valid {
            assert!(!iter.valid());
            continue;
        }
        assert!(iter.valid());

        let k = iter.key().unwrap();
        assert_eq!(tt.output.as_slice(), k.parse_key());
    }
}

#[test]
fn test_seek_for_prev() {
    let opts = get_test_table_options();
    let table = build_test_table("k", 10_000, opts).unwrap();
    let mut iter = table.iter(0);
    let data = vec![
        TestData {
            input: "abc".to_string().as_bytes().to_vec(),
            valid: false,
            output: "".to_string().as_bytes().to_vec(),
        },
        TestData {
            input: "k0100".to_string().as_bytes().to_vec(),
            valid: true,
            output: "k0100".to_string().as_bytes().to_vec(),
        },
        TestData {
            input: "k0100b".to_string().as_bytes().to_vec(),
            valid: true,
            output: "k0100".to_string().as_bytes().to_vec(),
        }, // Test case where we jump to next block.
        TestData {
            input: "k1234".to_string().as_bytes().to_vec(),
            valid: true,
            output: "k1234".to_string().as_bytes().to_vec(),
        },
        TestData {
            input: "k1234b".to_string().as_bytes().to_vec(),
            valid: true,
            output: "k1234".to_string().as_bytes().to_vec(),
        },
        TestData {
            input: "k9999".to_string().as_bytes().to_vec(),
            valid: true,
            output: "k9999".to_string().as_bytes().to_vec(),
        },
        TestData {
            input: "z".to_string().as_bytes().to_vec(),
            valid: true,
            output: "k9999".to_string().as_bytes().to_vec(),
        },
    ];

    for tt in data {
        iter.seek_to_key(Key::from(tt.input.clone()).with_timestamp(0).as_slice());

        if !tt.valid {
            assert!(!iter.valid());
            continue;
        }
        assert!(iter.valid());

        let k = iter.key().unwrap();
        assert_eq!(tt.output.as_slice(), k.parse_key());
    }
}

#[test]
fn test_iterate_from_start() {
    fn runner(n: usize) {
        let opts = get_test_table_options();
        let table = build_test_table("key", n, opts).unwrap();
        let mut iter = table.iter(0);
        iter.reset();
        iter.seek_to_first();
        // No need to do a Next.
        // ti.Seek brings us to the first key >= "". Essentially a SeekToFirst.
        let mut cnt = 0;

        while iter.valid() {
            let v = iter.val().unwrap();
            assert_eq!(
                format!("{}", cnt).as_bytes(),
                v.parse_value(),
                "n is  {}, cnt is {} k is {:?}",
                n,
                cnt,
                String::from_utf8(iter.key().unwrap().parse_key().to_vec()).unwrap()
            );
            assert_eq!('A' as u8, v.get_meta());
            cnt += 1;
            iter.next();
        }
        assert_eq!(cnt as usize, n);
    }

    for n in [99, 100, 101, 199, 200, 250, 9999, 10000] {
        runner(n);
    }
}

#[test]
fn test_iterate_from_end() {
    fn runner(n: usize) {
        let opts = get_test_table_options();
        let table = build_test_table("key", n, opts).unwrap();
        let mut iter = table.iter(0);
        iter.reset();
        iter.seek(Key::from("zzzzzz").with_timestamp(0).as_slice()); // Seek to end, an invalid element.
        assert!(!iter.valid());
        (0..n).rev().for_each(|i| {
            iter.prev();
            assert!(iter.valid());
            let v = iter.val().unwrap();
            assert_eq!(format!("{}", i).as_bytes(), v.parse_value());
            assert_eq!('A' as u8, v.get_meta());
        });
        iter.prev();
        assert!(!iter.valid());
    }

    for n in [99, 100, 101, 199, 200, 250, 9999, 10000] {
        runner(n);
    }
}

#[test]
fn test_table() {
    let opts = get_test_table_options();
    let table = build_test_table("key", 10_000, opts).unwrap();
    let mut iter = table.iter(0);
    let mut kid = 1010;
    let seek = Key::from(key("key", kid)).with_timestamp(0);
    iter.seek(seek.as_slice());
    while iter.valid() {
        let k = iter.key().unwrap();
        assert_eq!(k.parse_key(), key("key", kid).as_bytes());
        kid += 1;
        iter.next();
    }

    assert_eq!(kid, 10_000);

    iter.seek(Key::from(key("key", 99999)).with_timestamp(0).as_slice());
    assert!(!iter.valid());

    iter.seek(Key::from(key("key", -1)).with_timestamp(0).as_slice());
    assert!(iter.valid());
    let k = iter.key().unwrap();
    assert_eq!(k.parse_key(), key("key", 0).as_bytes());
}

#[test]
fn test_iterate_back_and_forth() {
    let opts = get_test_table_options();
    let table = build_test_table("key", 10_000, opts).unwrap();

    let seek = Key::from(key("key", 1010)).with_timestamp(0);
    let mut iter = table.iter(0);
    iter.seek(seek.as_slice());
    assert!(iter.valid());

    let k = iter.key().unwrap();
    assert_eq!(seek.as_slice(), k.as_slice());

    iter.prev();
    iter.prev();
    assert!(iter.valid());
    let k = iter.key().unwrap();
    assert_eq!(key("key", 1008).as_bytes(), k.parse_key());
    iter.next();
    iter.next();
    assert!(iter.valid());
    let k = iter.key().unwrap();
    assert_eq!(key("key", 1010).as_bytes(), k.parse_key());

    iter.seek(Key::from(key("key", 2000)).with_timestamp(0).as_slice());
    assert!(iter.valid());
    let k = iter.key().unwrap();
    assert_eq!(key("key", 2000).as_bytes(), k.parse_key());

    iter.prev();
    assert!(iter.valid());
    let k = iter.key().unwrap();
    assert_eq!(key("key", 1999).as_bytes(), k.parse_key());
    iter.seek_to_first();
    assert!(iter.valid());
    let k = iter.key().unwrap();
    assert_eq!(key("key", 0).as_bytes(), k.parse_key());
}

#[test]
fn test_concat_iterator_one_table() {
    let n = 2;

    let opts = get_test_table_options();
    let table1 = build_test_table("k", n, opts).unwrap();

    let mut t = ConcatTableIterator::new(vec![table1], 0);
    t.rewind();
    assert!(t.valid());
    let k = t.key().unwrap();
    assert_eq!("k0000".as_bytes(), k.parse_key());
    let v = t.val().unwrap();
    assert_eq!("0".as_bytes(), v.parse_value());
    assert_eq!('A' as u8, v.get_meta());
}

#[test]
fn test_uni_iterator() {
    let n = 10_000;
    let opts = get_test_table_options();
    let table = build_test_table("keya", n, opts).unwrap();
    let mut iter = table.iter(0);
    let mut cnt = 0;
    iter.next();
    for _ in 0..iter.count() {
        assert_eq!(
            format!("{}", cnt).as_bytes(),
            iter.val().unwrap().parse_value()
        );
        assert_eq!('A' as u8, iter.val().unwrap().get_meta());
        cnt += 1;
        iter.next();
    }
    assert_eq!(cnt, n);
}

#[test]
fn test_uni_iterator_reverse() {
    let n = 10_000;
    let opts = get_test_table_options();
    let table = build_test_table("keya", n, opts).unwrap();
    let mut iter = table.iter(REVERSED);
    let mut cnt = 0;
    iter.next();
    for _ in 0..iter.count() {
        assert_eq!(
            format!("{}", n - 1 - cnt).as_bytes(),
            iter.val().unwrap().parse_value()
        );
        assert_eq!('A' as u8, iter.val().unwrap().get_meta());
        cnt += 1;
        iter.next();
    }
    assert_eq!(cnt, n);
}

#[test]
fn test_concat_iterator() {
    let n = 10_000;
    let opts = get_test_table_options();
    let table1 = build_test_table("keya", n, opts.clone()).unwrap();
    let table2 = build_test_table("keyb", n, opts.clone()).unwrap();
    let table3 = build_test_table("keyc", n, opts).unwrap();

    {
        let mut it = ConcatTableIterator::new(vec![table1, table2, table3], 0);
        let mut cnt = 0;
        it.rewind();
        while it.valid() {
            assert_eq!(
                format!("{}", cnt % n).as_bytes(),
                it.val().unwrap().parse_value()
            );
            assert_eq!('A' as u8, it.val().unwrap().get_meta());
            cnt += 1;
            it.next();
        }

        assert_eq!(cnt, n * 3);
        it.seek(
            Key::from("a".as_bytes().to_vec())
                .with_timestamp(0)
                .as_slice(),
        );
        assert_eq!("keya0000".as_bytes(), it.key().unwrap().parse_key());
        assert_eq!("0".as_bytes(), it.val().unwrap().parse_value());

        it.seek(
            Key::from("keyb".as_bytes().to_vec())
                .with_timestamp(0)
                .as_slice(),
        );
        assert_eq!("keyb0000".as_bytes(), it.key().unwrap().parse_key());
        assert_eq!("0".as_bytes(), it.val().unwrap().parse_value());

        it.seek(
            Key::from("keyb9999b".as_bytes().to_vec())
                .with_timestamp(0)
                .as_slice(),
        );
        assert_eq!("keyc0000".as_bytes(), it.key().unwrap().parse_key());
        assert_eq!("0".as_bytes(), it.val().unwrap().parse_value());

        it.seek(
            Key::from("keyd".as_bytes().to_vec())
                .with_timestamp(0)
                .as_slice(),
        );
        assert!(!it.valid());
    }
}

#[test]
fn test_concat_iterator_reversed() {
    let n = 10_000;

    let opts = get_test_table_options();
    let table1 = build_test_table("keya", n, opts.clone()).unwrap();
    let table2 = build_test_table("keyb", n, opts.clone()).unwrap();
    let table3 = build_test_table("keyc", n, opts).unwrap();

    {
        let mut it = ConcatTableIterator::new(vec![table1, table2, table3], REVERSED);
        let mut cnt = 0;
        it.rewind();
        while it.valid() {
            assert_eq!(
                format!("{}", n - (cnt % n) - 1).as_bytes(),
                it.val().unwrap().parse_value()
            );
            assert_eq!('A' as u8, it.val().unwrap().get_meta());
            cnt += 1;
            it.next();
        }

        assert_eq!(cnt, n * 3);

        it.seek(
            Key::from("a".as_bytes().to_vec())
                .with_timestamp(0)
                .as_slice(),
        );
        assert!(!it.valid());

        it.seek(
            Key::from("keyb".as_bytes().to_vec())
                .with_timestamp(0)
                .as_slice(),
        );
        assert_eq!("keya9999".as_bytes(), it.key().unwrap().parse_key());
        assert_eq!("9999".as_bytes(), it.val().unwrap().parse_value());

        it.seek(
            Key::from("keyb9999b".as_bytes().to_vec())
                .with_timestamp(0)
                .as_slice(),
        );
        assert_eq!("keyb9999".as_bytes(), it.key().unwrap().parse_key());
        assert_eq!("9999".as_bytes(), it.val().unwrap().parse_value());

        it.seek(
            Key::from("keyd".as_bytes().to_vec())
                .with_timestamp(0)
                .as_slice(),
        );
        assert_eq!("keyc9999".as_bytes(), it.key().unwrap().parse_key());
        assert_eq!("9999".as_bytes(), it.val().unwrap().parse_value());
    }
}

#[test]
fn test_merging_iterator() {
    let opts = get_test_table_options();
    let tbl1: Table = build_table(
        vec![
            KV {
                index: 0,
                data: vec!["k1".as_bytes().to_vec(), "a1".as_bytes().to_vec()],
            },
            KV {
                index: 1,
                data: vec!["k4".as_bytes().to_vec(), "a4".as_bytes().to_vec()],
            },
            KV {
                index: 2,
                data: vec!["k5".as_bytes().to_vec(), "a5".as_bytes().to_vec()],
            },
        ],
        opts.clone(),
    )
    .unwrap();

    let tbl2: Table = build_table(
        vec![
            KV {
                index: 0,
                data: vec!["k2".as_bytes().to_vec(), "b2".as_bytes().to_vec()],
            },
            KV {
                index: 1,
                data: vec!["k3".as_bytes().to_vec(), "b3".as_bytes().to_vec()],
            },
            KV {
                index: 2,
                data: vec!["k4".as_bytes().to_vec(), "b4".as_bytes().to_vec()],
            },
        ],
        opts,
    )
    .unwrap();

    let expected = vec![
        ("k1", "a1"),
        ("k2", "b2"),
        ("k3", "b3"),
        ("k4", "a4"),
        ("k5", "a5"),
    ];
    let it1 = tbl1.iter(0);
    let it2 = ConcatTableIterator::new(vec![tbl2], 0);
    let mut it = MergeTableIterator::new(
        vec![
            Box::new(TableIterator::Uni(it1)),
            Box::new(TableIterator::Concat(it2)),
        ],
        false,
    )
    .unwrap();

    let mut cnt = 0;
    it.rewind();
    while it.valid() {
        let k = it.key().unwrap();
        let v = it.val().unwrap();
        assert_eq!(expected[cnt].0.as_bytes(), k.parse_key());
        assert_eq!(expected[cnt].1.as_bytes(), v.parse_value());
        assert_eq!('A' as u8, v.get_meta());
        cnt += 1;
        it.next();
    }
    assert_eq!(cnt, expected.len());
    assert!(!it.valid());
}

#[test]
fn test_merging_iterator_reversed() {
    let opts = get_test_table_options();
    let tbl1: Table = build_table(
        vec![
            KV {
                index: 0,
                data: vec!["k1".as_bytes().to_vec(), "a1".as_bytes().to_vec()],
            },
            KV {
                index: 1,
                data: vec!["k2".as_bytes().to_vec(), "a2".as_bytes().to_vec()],
            },
            KV {
                index: 2,
                data: vec!["k4".as_bytes().to_vec(), "a4".as_bytes().to_vec()],
            },
            KV {
                index: 3,
                data: vec!["k5".as_bytes().to_vec(), "a5".as_bytes().to_vec()],
            },
        ],
        opts.clone(),
    )
    .unwrap();

    let tbl2: Table = build_table(
        vec![
            KV {
                index: 0,
                data: vec!["k1".as_bytes().to_vec(), "b2".as_bytes().to_vec()],
            },
            KV {
                index: 1,
                data: vec!["k3".as_bytes().to_vec(), "b3".as_bytes().to_vec()],
            },
            KV {
                index: 2,
                data: vec!["k4".as_bytes().to_vec(), "b4".as_bytes().to_vec()],
            },
            KV {
                index: 3,
                data: vec!["k5".as_bytes().to_vec(), "b5".as_bytes().to_vec()],
            },
        ],
        opts,
    )
    .unwrap();

    let expected = vec![
        ("k5", "a5"),
        ("k4", "a4"),
        ("k3", "b3"),
        ("k2", "a2"),
        ("k1", "a1"),
    ];

    let it1 = tbl1.iter(REVERSED);
    let it2 = ConcatTableIterator::new(vec![tbl2], REVERSED);
    let mut it = MergeTableIterator::new(
        vec![
            Box::new(TableIterator::Uni(it1)),
            Box::new(TableIterator::Concat(it2)),
        ],
        true,
    )
    .unwrap();

    let mut cnt = 0;
    it.rewind();
    while it.valid() {
        let k = it.key().unwrap();
        let v = it.val().unwrap();
        assert_eq!(expected[cnt].0.as_bytes(), k.parse_key());
        assert_eq!(expected[cnt].1.as_bytes(), v.parse_value());
        assert_eq!('A' as u8, v.get_meta());
        cnt += 1;
        it.next();
    }
    assert_eq!(cnt, expected.len());
    assert!(!it.valid());
}

#[test]
fn test_merging_iterator_take_one() {
    let opts = get_test_table_options();
    let tbl1: Table = build_table(
        vec![
            KV {
                index: 0,
                data: vec!["k1".as_bytes().to_vec(), "a1".as_bytes().to_vec()],
            },
            KV {
                index: 1,
                data: vec!["k2".as_bytes().to_vec(), "a2".as_bytes().to_vec()],
            },
        ],
        opts.clone(),
    )
    .unwrap();

    let tbl2: Table = build_table(
        vec![KV {
            index: 0,
            data: vec!["l1".as_bytes().to_vec(), "b1".as_bytes().to_vec()],
        }],
        opts,
    )
    .unwrap();

    let it1 = ConcatTableIterator::new(vec![tbl1], 0);
    let it2 = ConcatTableIterator::new(vec![tbl2], 0);
    let mut it = MergeTableIterator::new(
        vec![
            Box::new(TableIterator::Concat(it1)),
            Box::new(TableIterator::Concat(it2)),
        ],
        false,
    )
    .unwrap();

    it.rewind();
    assert!(it.valid());
    let k = it.key().unwrap();
    let v = it.val().unwrap();
    assert_eq!("k1".to_string().as_bytes(), k.as_bytes());
    assert_eq!("a1".to_string(), v.to_string());
    assert_eq!('A' as u8, v.get_meta());
    it.next();

    assert!(it.valid());
    let k = it.key().unwrap();
    let v = it.val().unwrap();
    assert_eq!("k2".to_string().as_bytes(), k.as_bytes());
    assert_eq!("a2".to_string(), v.to_string());
    assert_eq!('A' as u8, v.get_meta());
    it.next();

    assert!(it.valid());
    let k = it.key().unwrap();
    let v = it.val().unwrap();
    assert_eq!("l1".to_string().as_bytes(), k.as_bytes());
    assert_eq!("b1".to_string(), v.to_string());
    assert_eq!('A' as u8, v.get_meta());
    it.next();

    assert!(!it.valid());
}

#[test]
fn test_merging_iterator_take_two() {
    let opts = get_test_table_options();
    let tbl1: Table = build_table(
        vec![KV {
            index: 0,
            data: vec!["l1".as_bytes().to_vec(), "b1".as_bytes().to_vec()],
        }],
        opts.clone(),
    )
    .unwrap();

    let tbl2: Table = build_table(
        vec![
            KV {
                index: 0,
                data: vec!["k1".as_bytes().to_vec(), "a1".as_bytes().to_vec()],
            },
            KV {
                index: 1,
                data: vec!["k2".as_bytes().to_vec(), "a2".as_bytes().to_vec()],
            },
        ],
        opts,
    )
    .unwrap();

    let it1 = ConcatTableIterator::new(vec![tbl1], 0);
    let it2 = ConcatTableIterator::new(vec![tbl2], 0);
    let mut it = MergeTableIterator::new(
        vec![
            Box::new(TableIterator::Concat(it1)),
            Box::new(TableIterator::Concat(it2)),
        ],
        false,
    )
    .unwrap();

    it.rewind();
    assert!(it.valid());

    let k = it.key().unwrap();
    let v = it.val().unwrap();
    assert_eq!(b"k1", k.as_bytes());
    assert_eq!("a1".to_string(), v.to_string());
    assert_eq!('A' as u8, v.get_meta());
    it.next();

    assert!(it.valid());
    let k = it.key().unwrap();
    let v = it.val().unwrap();
    assert_eq!(b"k2", k.as_bytes());
    assert_eq!("a2".to_string(), v.to_string());
    assert_eq!('A' as u8, v.get_meta());
    it.next();

    assert!(it.valid());
    let k = it.key().unwrap();
    let v = it.val().unwrap();
    assert_eq!(b"l1", k.as_bytes());
    assert_eq!("b1".to_string(), v.to_string());
    assert_eq!('A' as u8, v.get_meta());
    it.next();

    assert!(!it.valid());
}
