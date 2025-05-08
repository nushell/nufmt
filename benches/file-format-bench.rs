use criterion::{criterion_group, criterion_main, Criterion};
use nu_formatter::{config::Config, format_single_file, Mode};
use std::path::PathBuf;

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("Format massive nu", |b| {
        b.iter(|| {
            format_single_file(
                PathBuf::from("./benches/example.nu"),
                &Config::default(),
                &Mode::Normal,
            )
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
