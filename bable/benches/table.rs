use std::path::PathBuf;

use bable::{
    cache::{BlockCache, CacheOptions, IndexCache},
    BableIterator, Builder, Flag, MergeTableIterator, Options, RefCounter, SimpleBuilder, Table,
    TableBuilder, TableIterator,
};
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

fn get_table_for_bench<B: TableBuilder>(count: usize) -> Table {
    let opts = Options::default_with_pool(AllocatorPool::new(1))
        .set_compression(vpb::Compression::new())
        .set_block_cache(BlockCache::new(CacheOptions::new(1000000 * 10, 1000000)).unwrap());

    let mut builder = B::new(RefCounter::new(opts)).unwrap();
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
    c.bench_function("bench table simple builder", |b| {
        const KEY_COUNT: usize = 1300000; // about 64MB

        let mut key_list = vec![];
        for i in 0..KEY_COUNT {
            let k = Key::from(format!("{:032}", i));
            key_list.push(k);
        }

        let vs = Value::from(rand_value());

        let opt = RefCounter::new(
            Options::default_with_pool(AllocatorPool::new(1))
                .set_table_size(5 << 20)
                .set_block_size(4 * 1024)
                .set_compression(vpb::Compression::new()),
        );

        b.iter(|| {
            let mut builder = SimpleBuilder::new(opt.clone()).unwrap();
            key_list.iter().take(KEY_COUNT).for_each(|k| {
                builder.insert(k, &vs, 0);
            });

            builder.build().unwrap();
        });
    });

    c.bench_function("bench table builder no compression", |b| {
        const KEY_COUNT: usize = 1300000; // about 64MB

        let mut key_list = vec![];
        for i in 0..KEY_COUNT {
            let k = Key::from(format!("{:032}", i));
            key_list.push(k);
        }

        let vs = Value::from(rand_value());

        let opt = RefCounter::new(
            Options::default_with_pool(AllocatorPool::new(1))
                .set_table_size(5 << 20)
                .set_block_size(4 * 1024)
                .set_compression(vpb::Compression::new()),
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
            Options::default_with_pool(AllocatorPool::new(1))
                .set_compression(vpb::Compression::snappy()),
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
            Options::default_with_pool(AllocatorPool::new(1))
                .set_compression(vpb::Compression::lz4()),
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
            Options::default_with_pool(AllocatorPool::new(1))
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
            Options::default_with_pool(AllocatorPool::new(1))
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
            Options::default_with_pool(AllocatorPool::new(1))
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
            Options::default_with_pool(AllocatorPool::new(1))
                .set_index_cache(IndexCache::new(CacheOptions::new(1000, 1 << 20)).unwrap())
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
        let tbl = get_table_for_bench::<Builder>(n);
        b.iter(|| {
            let mut it = tbl.iter(Flag::NONE);
            it.seek_to_first();
            while it.valid() {
                it.next();
            }
        });
    });

    c.bench_function("bench table read and build (simple builder)", |b| {
        let n = 5 * (1e6 as usize);
        let tbl = get_table_for_bench::<SimpleBuilder>(n);
        b.iter(|| {
            let mut it = tbl.iter(Flag::NONE);
            let mut builder = SimpleBuilder::new(RefCounter::new(
                Options::default_with_pool(AllocatorPool::new(1))
                    .set_compression(vpb::Compression::new())
                    .set_block_cache(
                        BlockCache::new(CacheOptions::new(1000000 * 10, 1000000)).unwrap(),
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

    c.bench_function("bench table read and build", |b| {
        let n = 5 * (1e6 as usize);
        let tbl = get_table_for_bench::<Builder>(n);
        b.iter(|| {
            let mut it = tbl.iter(Flag::NONE);
            let mut builder = Builder::new(RefCounter::new(
                Options::default_with_pool(AllocatorPool::new(1))
                    .set_compression(vpb::Compression::new())
                    .set_block_cache(
                        BlockCache::new(CacheOptions::new(1000000 * 10, 1000000)).unwrap(),
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
    const M: usize = 5; //num of tables
    const N: usize = 5 * (1e6 as usize);
    const TABLE_SIZE: usize = N / M;
    c.bench_function("bench table merged (simple builder)", |b| {
        b.iter_batched(
            || {
                let cache = BlockCache::new(CacheOptions::new(1000000 * 10, 1000000)).unwrap();
                let mut tables = vec![];
                for i in 0..M {
                    let mut filename = PathBuf::new();
                    filename.push(std::env::temp_dir());
                    filename.push(rng.next_u32().to_string());
                    filename.set_extension("sst");
                    let opts = RefCounter::new(
                        Options::default_with_pool(AllocatorPool::new(1))
                            .set_compression(vpb::Compression::new())
                            .set_block_size(4 * 1024)
                            .set_block_cache(cache.clone()),
                    );
                    let mut builder = SimpleBuilder::new(opts).unwrap();
                    for j in 0..TABLE_SIZE {
                        let id = j * M + i;
                        let k = format!("{:016x}", id);
                        let v = format!("{}", id);
                        builder.insert(&Key::from(k), &Value::from(v), 0);
                    }
                    let tbl = Table::create_table(filename, builder).unwrap();
                    tables.push(tbl);
                }
                tables
            },
            |tables| {
                let mut iters = vec![];
                for tbl in tables {
                    iters.push(Box::new(TableIterator::Uni(tbl.iter(Flag::NONE))));
                }
                let mut iter = MergeTableIterator::new(iters, false).unwrap();
                iter.rewind();
                while iter.valid() {
                    iter.next();
                }
            },
            BatchSize::SmallInput,
        );
    });

    c.bench_function("bench table merged", |b| {
        b.iter_batched(
            || {
                let cache = BlockCache::new(CacheOptions::new(1000000 * 10, 1000000)).unwrap();
                let mut tables = vec![];
                for i in 0..M {
                    let mut filename = PathBuf::new();
                    filename.push(std::env::temp_dir());
                    filename.push(rng.next_u32().to_string());
                    filename.set_extension("sst");
                    let opts = RefCounter::new(
                        Options::default_with_pool(AllocatorPool::new(1))
                            .set_compression(vpb::Compression::new())
                            .set_block_size(4 * 1024)
                            .set_block_cache(cache.clone()),
                    );
                    let mut builder = Builder::new(opts).unwrap();
                    for j in 0..TABLE_SIZE {
                        let id = j * M + i;
                        let k = format!("{:016x}", id);
                        let v = format!("{}", id);
                        builder.insert(&Key::from(k), &Value::from(v), 0);
                    }
                    let tbl = Table::create_table(filename, builder).unwrap();
                    tables.push(tbl);
                }
                tables
            },
            |tables| {
                let mut iters = vec![];
                for tbl in tables {
                    iters.push(Box::new(TableIterator::Uni(tbl.iter(Flag::NONE))));
                }
                let mut iter = MergeTableIterator::new(iters, false).unwrap();
                iter.rewind();
                while iter.valid() {
                    iter.next();
                }
            },
            BatchSize::SmallInput,
        );
    });

    c.bench_function("bench random read", |b| {
        let n = 5 * (1e6 as usize);
        let tbl = get_table_for_bench::<Builder>(n);
        let mut it = tbl.iter(Flag::NONE);
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
