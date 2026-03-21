#[cfg(feature = "thread-safe")]
use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
#[cfg(not(feature = "thread-safe"))]
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use fluxdi::{Injector, Provider, Shared};

#[derive(Debug)]
struct CachedService(u64);

#[derive(Debug)]
struct TransientService(u64);

#[cfg(feature = "thread-safe")]
#[derive(Debug)]
struct ConcurrentService(u64);

fn bench_cached_resolve(c: &mut Criterion) {
    let injector = Injector::root();
    injector.provide::<CachedService>(Provider::root(|_| Shared::new(CachedService(42))));
    let _ = injector.resolve::<CachedService>();

    c.bench_function("resolve_cached", |b| {
        b.iter(|| {
            let value = injector.resolve::<CachedService>();
            black_box(value.0)
        });
    });
}

fn bench_transient_resolve(c: &mut Criterion) {
    let injector = Injector::root();
    injector.provide::<TransientService>(Provider::transient(|_| Shared::new(TransientService(7))));

    c.bench_function("resolve_transient", |b| {
        b.iter(|| {
            let value = injector.resolve::<TransientService>();
            black_box(value.0)
        });
    });
}

#[cfg(feature = "thread-safe")]
fn bench_concurrent_resolve(c: &mut Criterion) {
    use std::sync::{Arc, Barrier};
    use std::thread;

    let mut group = c.benchmark_group("resolve_concurrent");
    for workers in [2usize, 4, 8, 16, 32] {
        group.bench_with_input(
            BenchmarkId::from_parameter(workers),
            &workers,
            |b, &workers| {
                let injector = Arc::new(Injector::root());
                injector.provide::<ConcurrentService>(Provider::root(|_| {
                    Shared::new(ConcurrentService(11))
                }));
                let _ = injector.resolve::<ConcurrentService>();

                b.iter(|| {
                    let barrier = Arc::new(Barrier::new(workers));
                    let handles: Vec<_> = (0..workers)
                        .map(|_| {
                            let injector = Arc::clone(&injector);
                            let barrier = Arc::clone(&barrier);
                            thread::spawn(move || {
                                barrier.wait();
                                for _ in 0..64 {
                                    let value = injector.resolve::<ConcurrentService>();
                                    black_box(value.0);
                                }
                            })
                        })
                        .collect();

                    for handle in handles {
                        handle.join().unwrap();
                    }
                });
            },
        );
    }
    group.finish();
}

#[cfg(not(feature = "thread-safe"))]
fn bench_concurrent_resolve(_c: &mut Criterion) {}

fn bench_provider_registration(c: &mut Criterion) {
    c.bench_function("provider_registration", |b| {
        b.iter(|| {
            let injector = Injector::root();
            injector.provide::<u64>(Provider::transient(|_| Shared::new(1u64)));
            black_box(injector);
        });
    });
}

fn bench_resolve_with_decorator(c: &mut Criterion) {
    let injector = Injector::root();
    injector.provide::<CachedService>(
        Provider::root(|_| Shared::new(CachedService(42))).with_decorator(|inner| inner),
    );
    let _ = injector.resolve::<CachedService>();

    c.bench_function("resolve_with_noop_decorator", |b| {
        b.iter(|| {
            let value = injector.resolve::<CachedService>();
            black_box(value.0)
        });
    });
}

fn bench_resolve_decorator_baseline(c: &mut Criterion) {
    let mut group = c.benchmark_group("resolve_decorator");
    group.bench_function("without_decorator", |b| {
        let injector = Injector::root();
        injector.provide::<CachedService>(Provider::root(|_| Shared::new(CachedService(42))));
        let _ = injector.resolve::<CachedService>();
        b.iter(|| {
            let value = injector.resolve::<CachedService>();
            black_box(value.0)
        });
    });
    group.bench_function("with_noop_decorator", |b| {
        let injector = Injector::root();
        injector.provide::<CachedService>(
            Provider::root(|_| Shared::new(CachedService(42))).with_decorator(|inner| inner),
        );
        let _ = injector.resolve::<CachedService>();
        b.iter(|| {
            let value = injector.resolve::<CachedService>();
            black_box(value.0)
        });
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_cached_resolve,
    bench_transient_resolve,
    bench_concurrent_resolve,
    bench_provider_registration,
    bench_resolve_with_decorator,
    bench_resolve_decorator_baseline
);
criterion_main!(benches);
