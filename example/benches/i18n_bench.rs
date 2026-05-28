use criterion::{criterion_group, criterion_main, Criterion};
use i18n::*;
use std::hint::black_box;

#[allow(dead_code)]
mod i18n {
    include!(concat!(env!("OUT_DIR"), "/i18n_gen.rs"));
}

fn bench_all(c: &mut Criterion) {
    set_lang(Lang::EnUs);

    c.bench_function("pure text (&'static str)", |b| {
        b.iter(|| black_box(t!(settings())))
    });

    c.bench_function("variable interpolation (&str param)", |b| {
        b.iter(|| black_box(t!(hello("World"))))
    });

    c.bench_function("variable interpolation (long &str)", |b| {
        b.iter(|| {
            black_box(t!(hello(
                "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*()_+"
            )))
        })
    });

    c.bench_function("select numeric (one)", |b| {
        b.iter(|| black_box(t!(files(1))))
    });

    c.bench_function("select numeric (other)", |b| {
        b.iter(|| black_box(t!(files(42))))
    });

    c.bench_function("select string gender", |b| {
        b.iter(|| black_box(t!(user_greeting("male"))))
    });

    c.bench_function("message reference + free var", |b| {
        b.iter(|| black_box(t!(welcome_user("Jane"))))
    });

    c.bench_function("term reference (parameterized)", |b| {
        b.iter(|| black_box(t!(about_brand("genitive"))))
    });

    c.bench_function("attribute access", |b| {
        b.iter(|| black_box(t!(save__tooltip("doc"))))
    });

    c.bench_function("ordinal-like numeric selector", |b| {
        b.iter(|| black_box(t!(finish_place(3))))
    });

    c.bench_function("builtin func NUMBER (locale-aware)", |b| {
        b.iter(|| black_box(t!(dpi_ratio(Number::new(96.0).minimum_fraction_digits(2)))))
    });

    c.bench_function("builtin func DATETIME (locale-aware)", |b| {
        b.iter(|| black_box(t!(today_is(DateTime::new(0).year(2024).month(5).day(17)))))
    });

    c.bench_function("get_locale dispatch", |b| {
        b.iter(|| black_box(get_locale()))
    });
}

criterion_group!(benches, bench_all);
criterion_main!(benches);
