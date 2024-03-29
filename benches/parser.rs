use criterion::{criterion_group, criterion_main, Criterion};
use lenar::parser;

pub fn parser_benchmark(c: &mut Criterion) {
    use parser::*;

    let code = r#"
        let test = { { "test" } };
    "#
    .repeat(5000);

    let mut group = c.benchmark_group("sample-size-example");
    group.significance_level(0.1).sample_size(10);

    group.bench_function("parse 5000 lines", |b| b.iter(|| Parser::new(&code)));
}

criterion_group!(benches, parser_benchmark);
criterion_main!(benches);
