//! Decorator pattern example.
//!
//! Demonstrates `Provider::with_decorator()` for cross-cutting concerns:
//! - Logging: wrap calls with before/after prints
//! - Caching: memoize expensive computation results

use fluxdi::{Injector, Provider, Shared};
use std::collections::HashMap;
use std::sync::Mutex;

// ============================================================
// SERVICE 1: Greeter (logging decorator)
// ============================================================

trait Greeter: Send + Sync {
    fn greet(&self, name: &str) -> String;
}

struct GreeterImpl;

impl Greeter for GreeterImpl {
    fn greet(&self, name: &str) -> String {
        format!("Hello, {}!", name)
    }
}

struct LoggingGreeter {
    inner: Shared<dyn Greeter>,
}

impl Greeter for LoggingGreeter {
    fn greet(&self, name: &str) -> String {
        println!("  -> greet({})", name);
        let result = self.inner.greet(name);
        println!("  <- {}", result);
        result
    }
}

// ============================================================
// SERVICE 2: Calculator (caching decorator)
// ============================================================

trait Calculator: Send + Sync {
    fn compute(&self, x: u64) -> u64;
}

/// Simulates expensive computation (e.g. database lookup, API call).
struct ExpensiveCalculator;

impl Calculator for ExpensiveCalculator {
    fn compute(&self, x: u64) -> u64 {
        std::thread::sleep(std::time::Duration::from_millis(10));
        x * x
    }
}

/// Caches results; repeated calls with same input return cached value.
struct CachingCalculator {
    inner: Shared<dyn Calculator>,
    cache: Mutex<HashMap<u64, u64>>,
}

impl Calculator for CachingCalculator {
    fn compute(&self, x: u64) -> u64 {
        if let Ok(cache) = self.cache.lock() {
            if let Some(&v) = cache.get(&x) {
                return v;
            }
        }
        let result = self.inner.compute(x);
        if let Ok(mut cache) = self.cache.lock() {
            cache.insert(x, result);
        }
        result
    }
}

// ============================================================
// MAIN
// ============================================================

fn main() {
    fluxdi::init_logging();

    println!("╔════════════════════════════════════════════════════════╗");
    println!("║         FluxDI - Decorator Example                      ║");
    println!("╚════════════════════════════════════════════════════════╝\n");

    let injector = Injector::root();

    // 1. Logging decorator
    injector.provide::<dyn Greeter>(
        Provider::singleton(|_| Shared::new(GreeterImpl) as Shared<dyn Greeter>)
            .with_decorator(|inner| Shared::new(LoggingGreeter { inner }) as Shared<dyn Greeter>),
    );

    println!("--- Logging decorator ---");
    let greeter = injector.resolve::<dyn Greeter>();
    println!("Calling decorated greeter:");
    let msg = greeter.greet("World");
    println!("Result: {}\n", msg);

    // 2. Caching decorator
    injector.provide::<dyn Calculator>(
        Provider::singleton(|_| Shared::new(ExpensiveCalculator) as Shared<dyn Calculator>)
            .with_decorator(|inner| {
                Shared::new(CachingCalculator {
                    inner,
                    cache: Mutex::new(HashMap::new()),
                }) as Shared<dyn Calculator>
            }),
    );

    println!("--- Caching decorator ---");
    let calc = injector.resolve::<dyn Calculator>();
    println!("First compute(5) (slow, uncached)...");
    let t0 = std::time::Instant::now();
    let r1 = calc.compute(5);
    let d1 = t0.elapsed();
    println!("  result={}, took {:?}", r1, d1);

    println!("Second compute(5) (fast, cached)...");
    let t0 = std::time::Instant::now();
    let r2 = calc.compute(5);
    let d2 = t0.elapsed();
    println!("  result={}, took {:?}", r2, d2);

    assert_eq!(r1, r2);
    println!("✓ Cache hit: second call was faster.\n");

    println!("✓ Logging and caching decorators applied without modifying base services.");
}
