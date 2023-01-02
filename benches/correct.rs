use criterion::{criterion_group, criterion_main, Criterion};
use tifloats::{tifloat, Float, TIFloat};

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("add", |b| {
        b.iter(|| {
            let large = tifloat!(0x10000000000000 * 10 ^ 5);
            let neg_small = tifloat!(-0x10000000000000 * 10 ^ 4);

            assert_eq!(
                large.try_add(&neg_small).ok().unwrap(),
                tifloat!(0x90000000000000 * 10 ^ 4)
            );
        })
    });

    c.bench_function("mul", |b| {
        b.iter(|| {
            let large = tifloat!(0x10000000000000 * 10 ^ 5);
            let neg_small = tifloat!(-0x10000000000000 * 10 ^ 4);

            assert_eq!(
                large.try_mul(&neg_small).ok().unwrap(),
                tifloat!(-0x10000000000000 * 10 ^ 9)
            );
        })
    });

    c.bench_function("div", |b| {
        b.iter(|| {
            let large = tifloat!(0x10000000000000 * 10 ^ 5);
            let neg_small = tifloat!(-0x10000000000000 * 10 ^ 4);

            assert_eq!(
                large.try_div(&neg_small).ok().unwrap(),
                tifloat!(0x10000000000000 * 10 ^ 1)
            );
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
