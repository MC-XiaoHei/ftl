use criterion::{criterion_group, criterion_main, Criterion};
use i18n::*;
use std::hint::black_box;

#[allow(dead_code)]
mod i18n {
    include!(concat!(env!("OUT_DIR"), "/i18n_gen.rs"));
}

fn bench_all(c: &mut Criterion) {
    set_lang(Lang::EnUs);

    c.bench_function("pure text", |b| b.iter(|| black_box(t!(settings()))));

    c.bench_function("translate with placeholder", |b| {
        b.iter(|| black_box(t!(hello("World"))))
    });

    c.bench_function("translate with builtin func NUMBER", |b| {
        b.iter(|| black_box(t!(dpi_ratio(96.0))))
    });

    c.bench_function("translate with builtin func DATETIME", |b| {
        b.iter(|| black_box(t!(today_is(0))))
    });

    c.bench_function("translate with custom builtin", |b| {
        b.iter(|| black_box(t!(test_add_10(Test::new(7)))))
    });
}

criterion_group!(benches, bench_all);
criterion_main!(benches);
