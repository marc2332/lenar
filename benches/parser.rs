use criterion::{criterion_group, criterion_main, Criterion, black_box};
use lenar::tokenizer;

pub fn parser_benchmark(c: &mut Criterion) {
    use tokenizer::*;

    let code = r#"
        var test = { { "test" } };
    "#.repeat(1000);

    let mut group = c.benchmark_group("sample-size-example");
    group.significance_level(0.1).sample_size(10);

    group.bench_function("parse 1000 lines", |b| b.iter(|| Tokenizer::from_str(&code)));
}


criterion_group!(benches, parser_benchmark);
criterion_main!(benches);