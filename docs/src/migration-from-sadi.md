# Migration From SaDi

This fork has been renamed from `sadi` to `fluxdi`.

## 1) Cargo dependency

Before:

```toml
[dependencies]
sadi = "..."
```

After:

```toml
[dependencies]
fluxdi = "..."
```

## 2) Imports

Before:

```rust
use sadi::{Injector, Provider, Shared};
```

After:

```rust
use fluxdi::{Injector, Provider, Shared};
```

## 3) Workspace path dependency

Before:

```toml
sadi = { path = "../../sadi" }
```

After:

```toml
fluxdi = { path = "../../fluxdi" }
```

## 4) Observability names

Tracing span and metric prefixes are now `fluxdi.*` and `fluxdi_*`.
