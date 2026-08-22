//! Benchmarks: mimalloc vs system allocator.

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

#[global_allocator]
static GLOBAL: rustfs_mimalloc::MiMalloc = rustfs_mimalloc::MiMalloc;

fn bench_alloc_dealloc(c: &mut Criterion) {
    let mut group = c.benchmark_group("alloc_dealloc");
    for size in [8, 64, 256, 1024, 4096, 65536] {
        group.bench_with_input(BenchmarkId::new("mimalloc", size), &size, |b, &size| {
            b.iter(|| {
                let layout = std::alloc::Layout::from_size_align(size, 8).unwrap();
                unsafe {
                    let ptr = std::alloc::alloc(layout);
                    std::hint::black_box(ptr);
                    std::alloc::dealloc(ptr, layout);
                }
            });
        });
    }
    group.finish();
}

fn bench_aligned_alloc(c: &mut Criterion) {
    let mut group = c.benchmark_group("aligned_alloc");
    for align in [8, 64, 256, 1024, 4096] {
        group.bench_with_input(BenchmarkId::new("mimalloc", align), &align, |b, &align| {
            b.iter(|| {
                let layout = std::alloc::Layout::from_size_align(128, align).unwrap();
                unsafe {
                    let ptr = std::alloc::alloc(layout);
                    std::hint::black_box(ptr);
                    std::alloc::dealloc(ptr, layout);
                }
            });
        });
    }
    group.finish();
}

fn bench_vec(c: &mut Criterion) {
    let mut group = c.benchmark_group("vec");
    group.bench_function("push_1000", |b| {
        b.iter(|| {
            let mut v = Vec::new();
            for i in 0i32..1000 {
                v.push(i);
            }
            std::hint::black_box(&v);
        });
    });
    group.bench_function("extend_10000", |b| {
        b.iter(|| {
            let mut v = Vec::with_capacity(10000);
            v.extend(0i32..10000);
            std::hint::black_box(&v);
        });
    });
    group.finish();
}

criterion_group!(benches, bench_alloc_dealloc, bench_aligned_alloc, bench_vec);
criterion_main!(benches);
