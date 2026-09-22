//! Confirms the sub-20ms target from SPEC.md §8 before treating `nucleo`
//! as a settled dependency choice rather than an assumption.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use emoji_data::search;

fn bench(c: &mut Criterion) {
    let queries = ["rocket", "face_palm", "grinning face", "xyz_no_match", "he"];
    c.bench_function("search across full dataset", |b| {
        b.iter(|| {
            for q in queries {
                black_box(search(black_box(q)));
            }
        })
    });
}

criterion_group!(benches, bench);
criterion_main!(benches);
