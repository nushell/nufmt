use criterion::{criterion_group, criterion_main, Criterion};
use nu_formatter::{config::Config, format_single_file};
use std::{io, path::PathBuf};

fn format_massive_nu(file: &PathBuf) -> io::Result<()> {
    Ok(format_single_file(file, &Config::default()))
}

fn criterion_benchmark(c: &mut Criterion) {
    let file = PathBuf::from("./benches/example.nu");
    c.bench_function("Format massive nu", |b| b.iter(|| format_massive_nu(&file)));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
