# Web Integrations

FluxDI currently ships integrations for:

- Axum (`fluxdi::axum`)
- Actix-web (`fluxdi::actix`)

Use extractors to resolve services directly in handlers:

- `fluxdi::axum::Resolved<T>`
- `fluxdi::actix::Resolved<T>`

Relevant examples:

- `examples/axum`
- `examples/axum-lifecycle`
- `examples/actix`
- `examples/dual-http-random-port`
