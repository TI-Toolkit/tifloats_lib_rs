use std::hint::black_box;
use criterion::{criterion_group, criterion_main, Criterion};
use tifloats::{tifloat, Float};

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("add", |b| {
        b.iter(|| {
            let large = tifloat!(0x10000000000000 * 10 ^ 5);
            let neg_small = tifloat!(-0x10000000000000 * 10 ^ 4);

            assert_eq!(
                (large + neg_small).ok().unwrap(),
                tifloat!(0x90000000000000 * 10 ^ 4)
            );
        })
    });

    c.bench_function("mul", |b| {
        b.iter(|| {
            let large = tifloat!(0x10000000000000 * 10 ^ 5);
            let neg_small = tifloat!(-0x10000000000000 * 10 ^ 4);

            assert_eq!(
                (large * neg_small).ok().unwrap(),
                tifloat!(-0x10000000000000 * 10 ^ 9)
            );
        })
    });

    c.bench_function("div", |b| {
        b.iter(|| {
            let large = tifloat!(0x10000000000000 * 10 ^ 5);
            let neg_small = tifloat!(-0x10000000000000 * 10 ^ 4);

            assert_eq!(
                (large / neg_small).ok().unwrap(),
                tifloat!(0x10000000000000 * 10 ^ 1)
            );
        })
    });

    c.bench_function("float_from_num", |b| {
        b.iter(|| {
            let n = black_box(tifloats::Float::from(12345));

            assert_eq!(n, tifloat!(0x12345000000000 * 10 ^ 4));
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
