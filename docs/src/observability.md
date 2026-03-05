# Observability

FluxDI exposes stable observability identifiers in `fluxdi::observability`.

## Tracing

Enable:

```toml
[dependencies]
fluxdi = { version = "1.1.0", features = ["tracing"] }
```

## Metrics / Prometheus

Enable metrics:

```toml
[dependencies]
fluxdi = { version = "1.1.0", features = ["metrics"] }
```

Prometheus export:

```toml
[dependencies]
fluxdi = { version = "1.1.0", features = ["prometheus"] }
```

Then call:

- `Injector::metrics_snapshot()`
- `Injector::prometheus_metrics()`

## OpenTelemetry

Enable:

```toml
[dependencies]
fluxdi = { version = "1.1.0", features = ["opentelemetry"] }
```

Helpers:

- `fluxdi::opentelemetry_layer(...)`
- `fluxdi::try_init_opentelemetry(...)`
