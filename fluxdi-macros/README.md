# fluxdi-macros

Procedural macros for FluxDI.

This crate currently exposes one derive macro:

- `#[derive(Injectable)]`

## Recommended Usage

Use macros via `fluxdi`:

```toml
[dependencies]
fluxdi = { version = "1.2.0", features = ["macros"] }
```

Then import from `fluxdi`:

```rust
use fluxdi::Injectable;
```

## What `Injectable` Generates

For a struct:

```rust
#[derive(Injectable)]
struct AppService {
    dep: Shared<MyDep>,
}
```

the macro generates:

```rust
impl AppService {
    pub fn from_injector(injector: &fluxdi::Injector) -> fluxdi::Shared<Self> {
        fluxdi::Shared::new(Self {
            dep: injector.resolve::<MyDep>(),
        })
    }
}
```

## End-to-End Example

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

## Derive Constraints

`Injectable` currently supports:

- Named-field structs only.
- Non-generic structs only.
- Fields typed exactly as `Shared<T>`.

Not supported:

- Tuple structs and unit structs.
- Enums and unions.
- Generic structs (`struct Service<T> { ... }`).
- Non-`Shared<T>` fields in derived injection.

## Error Behavior

The generated method uses `injector.resolve::<T>()`, so missing providers panic at runtime.

If you need recoverable errors, create your own constructor and use `try_resolve::<T>()` manually.
