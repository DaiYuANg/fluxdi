# Decorators

Use `Provider::with_decorator()` to wrap resolved instances with cross-cutting
behavior (logging, caching, retry) without modifying business services.

## Basic Usage

```rust
use fluxdi::{Injector, Provider, Shared};

trait Service: Send + Sync {
    fn do_work(&self) -> String;
}

struct RealService;
impl Service for RealService {
    fn do_work(&self) -> String { "done".to_string() }
}

// Logging decorator
struct LoggingService {
    inner: Shared<dyn Service>,
}
impl Service for LoggingService {
    fn do_work(&self) -> String {
        println!("before");
        let r = self.inner.do_work();
        println!("after: {}", r);
        r
    }
}

let injector = Injector::root();
injector.provide::<dyn Service>(
    Provider::singleton(|_| Shared::new(RealService) as Shared<dyn Service>)
        .with_decorator(|inner| Shared::new(LoggingService { inner }) as Shared<dyn Service>),
);

let svc = injector.resolve::<dyn Service>();
svc.do_work();  // Logs "before" and "after"
```

## Multiple Decorators

Chain decorators; they run in order (base → first → second):

```rust
provider
    .with_decorator(logging_decorator)
    .with_decorator(caching_decorator)
```

## Order and Scope

- Decorators run each time the **factory** runs.
- For singletons, the factory runs once, so each decorator runs once.
- For transients, the factory runs per resolve, so decorators run per resolve.
- Scope (singleton, transient, etc.) is preserved.

## Example

See `examples/decorator` for a runnable demo with logging and caching decorators.
