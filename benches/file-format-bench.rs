use criterion::{Criterion, criterion_group, criterion_main};
use nu_formatter::{config::Config, format_string};

fn criterion_benchmark(c: &mut Criterion) {
    let input = include_str!("example.nu");
    let config = Config::default();

    c.bench_function("Format massive nu", |b| {
        b.iter(|| {
            format_string(input, &config).expect("benchmark fixture should contain valid Nushell")
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
