use criterion::{criterion_group, criterion_main, Criterion};

pub fn analysis_benhmark(c: &mut Criterion) {
    c.bench_function("stub benchmark", |b| b.iter(|| {}));
}

criterion_group!(benches, analysis_benhmark);
criterion_main!(benches);
