//! Parse + render benchmarks on generated documents (DESIGN.md §4, §14).
//! Run with `cargo bench -p scrald-core`.

use std::hint::black_box;
use std::path::PathBuf;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use scrald_core::generate::generate_document;
use scrald_core::parse_document;

fn bench_parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_document");
    // Big inputs: fewer samples keep a full run to a minute or two.
    group.sample_size(10);
    for words in [100_000, 250_000, 500_000] {
        let text = generate_document(words, 42);
        group.throughput(Throughput::Bytes(text.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(words), &text, |b, text| {
            b.iter(|| parse_document(PathBuf::from("bench.md"), black_box(text.as_bytes())))
        });
    }
    group.finish();
}

// Rust note: these are `macro_rules!` macros from criterion that generate the
// benchmark's `main` function, since benches use `harness = false`.
criterion_group!(benches, bench_parse);
criterion_main!(benches);
