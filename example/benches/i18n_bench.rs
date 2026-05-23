use criterion::{black_box, criterion_group, criterion_main, Criterion};

include!(concat!(env!("OUT_DIR"), "/i18n_gen.rs"));

fn bench_all(c: &mut Criterion) {
    set_lang(Lang::EnUs);

    c.bench_function("settings (pure text)", |b| {
        b.iter(|| black_box(t!(settings())))
    });

    c.bench_function("hello (short &str)", |b| {
        b.iter(|| black_box(t!(hello("World"))))
    });

    c.bench_function("hello (long &str)", |b| {
        b.iter(|| {
            black_box(t!(hello(
                "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*()_+"
            )))
        })
    });

    c.bench_function("files (select one)", |b| b.iter(|| black_box(t!(files(1)))));

    c.bench_function("files (select other)", |b| {
        b.iter(|| black_box(t!(files(42))))
    });

    c.bench_function("get_locale (id match)", |b| {
        b.iter(|| black_box(get_locale()))
    });
}

criterion_group!(benches, bench_all);
criterion_main!(benches);
