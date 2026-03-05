# Web Integrations

FluxDI currently ships integrations for:

- Axum (`fluxdi::axum`)
- Actix-web (`fluxdi::actix`)

Use extractors to resolve services directly in handlers:

- `fluxdi::axum::Resolved<T>`
- `fluxdi::actix::Resolved<T>`

## Scoped services in web requests

`Resolved<T>` resolves from the injector stored in framework state.
For request/task isolation (`Provider::scoped(...)`), create a runtime scope inside the handler:

```rust
let scoped = state.injector().create_scope();
let value = scoped.resolve::<MyScopedService>();
```

Inside one scope, scoped service instances are reused.
Across different requests/scopes, instances are isolated.

### Axum pattern

```rust
use axum::extract::State;
use fluxdi::axum::InjectorState;

async fn handler(State(state): State<InjectorState>) -> String {
    let scoped = state.injector().create_scope();
    let a = scoped.resolve::<RequestContext>();
    let b = scoped.resolve::<RequestContext>();
    format!(
        "scope_id={} same_within_scope={}",
        a.request_scope_id,
        fluxdi::Shared::ptr_eq(&a, &b)
    )
}
```

### Actix pattern

```rust
use actix_web::web;
use fluxdi::actix::InjectorState;

async fn handler(state: web::Data<InjectorState>) -> String {
    let scoped = state.injector().create_scope();
    let a = scoped.resolve::<RequestContext>();
    let b = scoped.resolve::<RequestContext>();
    format!(
        "scope_id={} same_within_scope={}",
        a.request_scope_id,
        fluxdi::Shared::ptr_eq(&a, &b)
    )
}
```

## Relevant examples

- `examples/axum`
- `examples/axum-lifecycle`
- `examples/actix`
- `examples/scoped-context`
- `examples/dual-http-random-port`
