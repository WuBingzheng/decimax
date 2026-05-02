use criterion::{
    BenchmarkGroup, BenchmarkId, Criterion, criterion_group, criterion_main, measurement::WallTime,
};
use std::env;
use std::hint::black_box;
use std::str::FromStr;

// rust_decimal
type RustDec = rust_decimal::prelude::Decimal;

// decimax
type MaxDec = decimax::Dec128;

fn do_bench_add(group: &mut BenchmarkGroup<'_, WallTime>, name: &str, scale: u32, iexp: u32) {
    let man = 10_i128.pow(iexp);

    // rust_decimal
    if iexp <= 28 {
        let a = RustDec::from_i128_with_scale(man, 0);
        let b = RustDec::from_i128_with_scale(man, scale);

        group.bench_with_input(
            BenchmarkId::new(format!("{name}:rust_decimal"), iexp),
            &(a, b),
            |b, i| b.iter(|| black_box(i.0 + i.1)),
        );
    }

    // decimax
    let a = MaxDec::from_parts(man, 0);
    let b = MaxDec::from_parts(man, scale);
    group.bench_with_input(
        BenchmarkId::new(format!("{name}:decimax"), iexp),
        &(a, b),
        |b, i| b.iter(|| black_box(i.0 + i.1)),
    );
}

fn bench_add(c: &mut Criterion, machine: &str, sample: usize) {
    let mut group = c.benchmark_group(format!("addition{machine}"));

    for iexp in (0..=36).step_by(sample) {
        do_bench_add(&mut group, "pure", 0, iexp);
        do_bench_add(&mut group, "rescale", 15, iexp);
    }

    // done
    group.finish();
}

fn bench_mul(c: &mut Criterion, machine: &str, sample: usize) {
    let mut group = c.benchmark_group(format!("multiplication{machine}"));

    for i in (0..=36).step_by(sample) {
        let man = 10_i128.pow(i) + 13;

        if i <= 28 {
            let a = RustDec::from_i128_with_scale(man, i);
            group.bench_with_input(BenchmarkId::new("rust_decimal", i), &(a, a), |b, i| {
                b.iter(|| black_box(i.0 * i.1))
            });
        }

        let a = MaxDec::from_parts(man, i.min(31));
        group.bench_with_input(BenchmarkId::new("decimax", i), &(a, a), |b, i| {
            b.iter(|| black_box(i.0 * i.1))
        });
    }

    // done
    group.finish();
}

fn do_bench_div(group: &mut BenchmarkGroup<'_, WallTime>, n_exp: u32, d_exp: u32, evenly: bool) {
    let n_man = 10_i128.pow(n_exp);
    let d_man = 10_i128.pow(d_exp);

    let name = if evenly { "evenly" } else { "non-evenly" };
    let extra = if evenly { 0 } else { 1 };

    // rust_decimal
    if n_exp <= 28 && d_exp <= 28 {
        let n = RustDec::from_i128_with_scale(n_man, 10);
        let d = RustDec::from_i128_with_scale(d_man + extra, 10);
        group.bench_with_input(
            BenchmarkId::new(format!("{name}:rust_decimal"), n_exp),
            &(n, d),
            |b, i| b.iter(|| black_box(i.0 / i.1)),
        );
    }

    // decimax
    let n = MaxDec::from_parts(n_man, 10);
    let d = MaxDec::from_parts(d_man + extra, 10);
    group.bench_with_input(
        BenchmarkId::new(format!("{name}:decimax"), n_exp),
        &(n, d),
        |b, i| b.iter(|| black_box(i.0 / i.1)),
    );
}

fn bench_div_by_small(c: &mut Criterion, machine: &str, sample: usize) {
    let mut group = c.benchmark_group(format!("division-by-small{machine}"));
    for i in (0..=36).step_by(sample) {
        do_bench_div(&mut group, i, 8, true);
        do_bench_div(&mut group, i, 8, false);
    }
    group.finish();
}

fn bench_div_by_big(c: &mut Criterion, machine: &str, sample: usize) {
    let mut group = c.benchmark_group(format!("division-by-big{machine}"));
    for i in (0..=36).step_by(sample) {
        do_bench_div(&mut group, i, 18, true);
        do_bench_div(&mut group, i, 18, false);
    }
    group.finish();
}

// entry
fn criterion_benchmark(c: &mut Criterion) {
    let machine = env::var("MACHINE").unwrap_or_default();
    let sample = env::var("SAMPLE")
        .map(|s| usize::from_str(&s).expect("invalid SAMPLE"))
        .unwrap_or(1);

    bench_add(c, &machine, sample);
    bench_mul(c, &machine, sample);
    bench_div_by_small(c, &machine, sample);
    bench_div_by_big(c, &machine, sample);
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
