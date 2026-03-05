# Providers And Resolution

FluxDI supports three scopes:

- `Provider::transient(...)` creates a new instance for each resolve.
- `Provider::root(...)` caches globally at root injector scope.
- `Provider::module(...)` caches at module injector scope.

Example:

```rust
use fluxdi::{Injector, Provider, Shared};

#[derive(Debug)]
struct Logger;

let injector = Injector::root();
injector.provide::<Logger>(Provider::root(|_| Shared::new(Logger)));

let logger = injector.resolve::<Logger>();
```

Prefer `try_resolve::<T>()` when you need recoverable errors:

```rust
let logger = injector.try_resolve::<Logger>()?;
```
