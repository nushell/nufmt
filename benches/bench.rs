use criterion::{criterion_group, criterion_main, Criterion};
use nufmt::{format_nu, Indentation};
use std::{fs, io};

/// You need a nu file called massive.nu in your project root
fn format_massive_nu(file: &str) -> io::Result<String> {
    Ok(format_nu(&file, Indentation::Default))
}

fn criterion_benchmark(c: &mut Criterion) {
    let file = fs::read_to_string("massive.nu").expect("massive.nu file in project directory");

    c.bench_function("Format massive nu", |b| b.iter(|| format_massive_nu(&file)));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
