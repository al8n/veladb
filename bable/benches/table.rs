use bable::{BableIterator, Builder, RefCounter, Table, TableOptions};
use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use rand::{thread_rng, Rng, RngCore};
use scopeguard::defer;

use rand::distributions::Alphanumeric;
use vpb::kvstructs::{Key, Value, ValueExt};

#[cfg(not(target_env = "msvc"))]
use tikv_jemallocator::Jemalloc;
use zallocator::pool::AllocatorPool;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

#[allow(dead_code)]
pub fn rand_value() -> Vec<u8> {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .collect::<Vec<_>>()
}

fn get_table_for_bench(count: usize) -> Table {
    let opts =
        TableOptions::default_with_pool(AllocatorPool::new(1)).set_compression(vpb::Compression {
            algo: vpb::CompressionAlgorithm::None,
            level: 0,
        });

    let mut builder = Builder::new(RefCounter::new(opts)).unwrap();
    let mut rng = thread_rng();

    let mut filename = std::env::temp_dir();
    filename.push(rng.gen::<u32>().to_string());
    filename.set_extension("sst");
    defer!(std::fs::remove_file(&filename).unwrap());

    (0..count).for_each(|i| {
        builder.insert(
            &Key::from(format!("{:016x}", i)),
            &Value::from(i.to_string()),
            0,
        );
    });

    Table::create_table(&filename, builder).unwrap()
}

fn bench_table_builder(c: &mut Criterion) {
    c.bench_function("bench table builder no compression", |b| {
        const KEY_COUNT: usize = 1300000; // about 64MB

        let mut key_list = vec![];
        for i in 0..KEY_COUNT {
            let k = Key::from(format!("{:032}", i));
            key_list.push(k);
        }

        let vs = Value::from(rand_value());

        let opt = RefCounter::new(
            TableOptions::default_with_pool(AllocatorPool::new(1)).set_compression(
                vpb::Compression {
                    algo: vpb::CompressionAlgorithm::None,
                    level: 0,
                },
            ),
        );

        b.iter(|| {
            let mut builder = Builder::new(opt.clone()).unwrap();
            key_list.iter().take(KEY_COUNT).for_each(|k| {
                builder.insert(k, &vs, 0);
            });

            builder.build().unwrap();
        });
    });

    c.bench_function("bench table builder snappy compression", |b| {
        const KEY_COUNT: usize = 1300000; // about 64MB

        let mut key_list = vec![];
        for i in 0..KEY_COUNT {
            let k = Key::from(format!("{:032}", i));
            key_list.push(k);
        }

        let vs = Value::from(rand_value());

        let opt = RefCounter::new(
            TableOptions::default_with_pool(AllocatorPool::new(1)).set_compression(
                vpb::Compression {
                    algo: vpb::CompressionAlgorithm::None,
                    level: 0,
                },
            ),
        );

        b.iter(|| {
            let mut builder = Builder::new(opt.clone()).unwrap();
            key_list.iter().take(KEY_COUNT).for_each(|k| {
                builder.insert(k, &vs, 0);
            });

            builder.build().unwrap();
        });
    });

    c.bench_function("bench table builder lz4 compression", |b| {
        const KEY_COUNT: usize = 1300000; // about 64MB

        let mut key_list = vec![];
        for i in 0..KEY_COUNT {
            let k = Key::from(format!("{:032}", i));
            key_list.push(k);
        }

        let vs = Value::from(rand_value());

        let opt = RefCounter::new(
            TableOptions::default_with_pool(AllocatorPool::new(1)).set_compression(
                vpb::Compression::new().set_algorithm(vpb::CompressionAlgorithm::Lz4),
            ),
        );

        b.iter(|| {
            let mut builder = Builder::new(opt.clone()).unwrap();
            key_list.iter().take(KEY_COUNT).for_each(|k| {
                builder.insert(k, &vs, 0);
            });

            builder.build().unwrap();
        });
    });

    c.bench_function("bench table builder zstd compression level 1", |b| {
        const KEY_COUNT: usize = 1300000; // about 64MB

        let mut key_list = vec![];
        for i in 0..KEY_COUNT {
            let k = Key::from(format!("{:032}", i));
            key_list.push(k);
        }

        let vs = Value::from(rand_value());

        let opt = RefCounter::new(
            TableOptions::default_with_pool(AllocatorPool::new(1))
                .set_compression(vpb::Compression::zstd(1)),
        );

        b.iter(|| {
            let mut builder = Builder::new(opt.clone()).unwrap();
            key_list.iter().take(KEY_COUNT).for_each(|k| {
                builder.insert(k, &vs, 0);
            });

            builder.build().unwrap();
        });
    });

    c.bench_function("bench table builder zstd compression level 3", |b| {
        const KEY_COUNT: usize = 1300000; // about 64MB

        let mut key_list = vec![];
        for i in 0..KEY_COUNT {
            let k = Key::from(format!("{:032}", i));
            key_list.push(k);
        }

        let vs = Value::from(rand_value());

        let opt = RefCounter::new(
            TableOptions::default_with_pool(AllocatorPool::new(1))
                .set_compression(vpb::Compression::zstd(3)),
        );

        b.iter(|| {
            let mut builder = Builder::new(opt.clone()).unwrap();
            key_list.iter().take(KEY_COUNT).for_each(|k| {
                builder.insert(k, &vs, 0);
            });

            builder.build().unwrap();
        });
    });

    c.bench_function("bench table builder zstd compression level 15", |b| {
        const KEY_COUNT: usize = 1300000; // about 64MB

        let mut key_list = vec![];
        for i in 0..KEY_COUNT {
            let k = Key::from(format!("{:032}", i));
            key_list.push(k);
        }

        let vs = Value::from(rand_value());

        let opt = RefCounter::new(
            TableOptions::default_with_pool(AllocatorPool::new(1))
                .set_compression(vpb::Compression::zstd(15)),
        );

        b.iter(|| {
            let mut builder = Builder::new(opt.clone()).unwrap();
            key_list.iter().take(KEY_COUNT).for_each(|k| {
                builder.insert(k, &vs, 0);
            });

            builder.build().unwrap();
        });
    });

    c.bench_function("bench table builder encryption", |b| {
        const KEY_COUNT: usize = 1300000; // about 64MB

        let mut key_list = vec![];
        for i in 0..KEY_COUNT {
            let k = Key::from(format!("{:032}", i));
            key_list.push(k);
        }

        let vs = Value::from(rand_value());

        let mut rng = thread_rng();
        let mut key = vec![0u8; 32];
        rng.fill_bytes(key.as_mut_slice());

        let opt = RefCounter::new(
            TableOptions::default_with_pool(AllocatorPool::new(1))
                .set_encryption(vpb::Encryption::aes(key)),
        );

        b.iter(|| {
            let mut builder = Builder::new(opt.clone()).unwrap();
            key_list.iter().take(KEY_COUNT).for_each(|k| {
                builder.insert(k, &vs, 0);
            });
            builder.build().unwrap();
        });
    });
}

fn bench_table(c: &mut Criterion) {
    c.bench_function("bench table read", |b| {
        let n = 5 * (1e6 as usize);
        let tbl = get_table_for_bench(n);
        b.iter(|| {
            let mut it = tbl.iter(0);
            it.seek_to_first();
            while it.valid() {
                it.next();
            }
        });
    });

    c.bench_function("bench table read and build", |b| {
        let n = 5 * (1e6 as usize);
        let tbl = get_table_for_bench(n);
        b.iter(|| {
            let mut it = tbl.iter(0);
            let mut builder = Builder::new(RefCounter::new(
                TableOptions::default_with_pool(AllocatorPool::new(1)).set_compression(
                    vpb::Compression {
                        algo: vpb::CompressionAlgorithm::None,
                        level: 0,
                    },
                ),
            ))
            .unwrap();
            it.seek_to_first();
            while it.valid() {
                builder.insert(
                    &it.key().unwrap().to_key(),
                    &it.val().unwrap().to_value(),
                    0,
                );
                it.next();
            }
            builder.build().unwrap();
        });
    });

    let mut rng = thread_rng();
    c.bench_function("bench random read", |b| {
        let n = 5 * (1e6 as usize);
        let tbl = get_table_for_bench(n);
        let mut it = tbl.iter(0);
        b.iter_batched(
            || {
                let i: usize = rng.gen_range(0..n);
                (Key::from(format!("{:016x}", i)), Value::from(i.to_string()))
            },
            |(k, v)| {
                it.seek(&k);
                assert!(it.valid());
                assert_eq!(it.val().unwrap().parse_value(), v.parse_value())
            },
            BatchSize::SmallInput,
        );
    });
}

criterion_group! {
    name = benches_table;
    config = Criterion::default().sample_size(10);
    targets = bench_table_builder, bench_table
}

criterion_main!(benches_table);
