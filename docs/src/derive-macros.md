# Derive Macros

FluxDI provides `#[derive(Injectable)]` through the `macros` feature.

Enable:

```toml
[dependencies]
fluxdi = { version = "1.2.0", features = ["macros"] }
```

Example:

```rust
use fluxdi::{Injectable, Injector, Provider, Shared};

#[derive(Debug)]
struct Clock {
    tick: u64,
}

#[derive(Injectable)]
struct AppService {
    clock: Shared<Clock>,
}

let injector = Injector::root();
injector.provide::<Clock>(Provider::root(|_| Shared::new(Clock { tick: 42 })));
injector.provide::<AppService>(Provider::root(|inj| AppService::from_injector(inj)));

let service = injector.resolve::<AppService>();
assert_eq!(service.clock.tick, 42);
```

`Injectable` generates:

- `impl YourType { pub fn from_injector(injector: &fluxdi::Injector) -> fluxdi::Shared<Self> }`
- One `injector.resolve::<T>()` call per field `Shared<T>`.

Current derive constraints:

- Only named-field structs are supported.
- Generic structs are not supported.
- Every injected field must be typed exactly as `Shared<T>`.
