# Observability

FluxDI exposes stable observability identifiers in `fluxdi::observability`.

## Logging

Enable:

```toml
[dependencies]
fluxdi = { version = "1.2.0", features = ["logging"] }
```

Initialize once at app startup:

```rust
fluxdi::try_init_logging().expect("failed to initialize logging");
```

`logging` reads `RUST_LOG` and defaults to `info` when unset.

For node-level diagnostics (registration, provider lookup, cache hit/miss, factory execution), set:

```bash
RUST_LOG=fluxdi=trace
```

## Tracing

Enable:

```toml
[dependencies]
fluxdi = { version = "1.2.0", features = ["tracing"] }
```

## Metrics / Prometheus

Enable metrics:

```toml
[dependencies]
fluxdi = { version = "1.2.0", features = ["metrics"] }
```

Prometheus export:

```toml
[dependencies]
fluxdi = { version = "1.2.0", features = ["prometheus"] }
```

Then call:

- `Injector::metrics_snapshot()`
- `Injector::prometheus_metrics()`

## OpenTelemetry

Enable:

```toml
[dependencies]
fluxdi = { version = "1.2.0", features = ["opentelemetry"] }
```

Helpers:

- `fluxdi::opentelemetry_layer(...)`
- `fluxdi::try_init_opentelemetry(...)`
