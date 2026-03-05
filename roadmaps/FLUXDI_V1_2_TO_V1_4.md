# FluxDI Product Roadmap (v1.2 -> v1.4)

This document defines an execution-focused roadmap for FluxDI after the rebrand.

## Release Goals

- Keep user-facing API simple (`configure` + unified bootstrap/shutdown).
- Improve developer experience before adding advanced runtime behaviors.
- Ship features in layers: DX -> composition -> runtime orchestration.

## Version Themes

- **v1.2**: Developer experience and testability.
- **v1.3**: Advanced composition capabilities.
- **v1.4**: Runtime lifecycle and scoped execution model.

## v1.2 (DX + Testability)

### Epic A - Derive Macros (`fluxdi-macros`)

Status: [~] In progress

Target outcomes:

- `#[derive(Injectable)]` for constructor-like wiring.
- Optional `#[inject]` field/param metadata for clarity.
- Reduced provider boilerplate in modules.

Planned tasks:

- [x] Create `fluxdi-macros` crate in workspace.
- [x] Implement `#[derive(Injectable)]` for struct constructors.
- [x] Generate compile errors with actionable messages.
- [ ] Add docs + examples for macro and manual provider parity.

Acceptance criteria:

- Macro-generated wiring works with sync providers.
- Works with `thread-safe` and default modes.
- Existing manual provider APIs remain fully supported.

### Epic B - Testing Override API

Status: [x] Completed

Target outcomes:

- Easy mock replacement in integration tests.
- No production behavior changes by default.

Planned tasks:

- [x] Add `override_provider::<T>(...)` API.
- [x] Add override semantics for `root/module/transient`.
- [x] Add conflict handling and deterministic override order.
- [x] Add tests with mock repository/service replacement.

Acceptance criteria:

- Override behavior is deterministic and documented.
- Clear error for missing target provider or invalid override state.

### Epic C - Error Suggestions

Status: [x] Completed

Target outcomes:

- Faster diagnosis for common user mistakes.

Planned tasks:

- [x] Add suggestions to `ServiceNotProvided`.
- [x] Add suggestions to `AsyncFactoryRequiresAsyncResolve`.
- [x] Add resolution-path hint for circular dependency errors.
- [x] Add examples of improved error output to docs.

Acceptance criteria:

- Core error kinds include at least one actionable suggestion.
- Existing error kind matching remains stable.

## v1.3 (Composition Capabilities)

### Epic D - Named/Tagged Bindings

Status: [x] Completed

Target outcomes:

- Support multiple implementations of one trait by key/tag.

Planned tasks:

- [x] Add `provide_named::<T>(name, provider)` API.
- [x] Add `try_resolve_named::<T>(name)` API.
- [x] Extend internal provider key model for typed+named lookup.
- [x] Add docs and examples for strategy pattern and multi-backend use.

Acceptance criteria:

- Same trait supports multiple named registrations.
- Resolve errors include missing name and type context.

### Epic E - Multi-binding

Status: [x] Completed

Target outcomes:

- Resolve a set/list of implementations for plugin pipelines.

Planned tasks:

- [x] Add `provide_into_set::<T>(provider)` API.
- [x] Add `try_resolve_all::<T>() -> Vec<Shared<T>>` API.
- [x] Define deterministic ordering strategy.
- [x] Add middleware/plugin chain example.

Acceptance criteria:

- Multi-binding works with trait objects.
- Ordering behavior is explicit and tested.

### Epic F - Graph Tooling

Status: [x] Completed

Target outcomes:

- Visualize and validate dependency graphs pre-runtime.

Planned tasks:

- [x] Add graph model export (`dependency_graph()`).
- [x] Add DOT and Mermaid text output.
- [x] Add `validate_graph()` with missing dependency and cycle checks.
- [x] Add CI-friendly command/example in docs.

Acceptance criteria:

- Graph export works without changing resolve behavior.
- Validation failures are actionable.

## v1.4 (Runtime Orchestration)

### Epic G - Scoped Context

Status: [x] Completed

Target outcomes:

- Add request/task-level scope for web and job workloads.

Planned tasks:

- [x] Add `create_scope()` API and scoped injector semantics.
- [x] Define scoped cache lifecycle and parent lookup behavior.
- [x] Integrate scope usage docs for Axum/Actix examples.
- [x] Add stress/concurrency tests for scoped resolve.

Acceptance criteria:

- Scoped services are isolated per scope.
- Root and module semantics remain backward-compatible.

### Epic H - Lifecycle Manager Extensions

Status: [ ] Not started

Target outcomes:

- Better startup/shutdown control for production environments.

Planned tasks:

- [ ] Add bootstrap options (`timeout`, `parallel_start`, rollback policy).
- [ ] Add structured lifecycle error aggregation.
- [ ] Add optional graceful stop timeout behavior.
- [ ] Document recommended patterns for long-running servers.

Acceptance criteria:

- Lifecycle options are optional and backwards-compatible.
- Failure behavior is deterministic and covered by tests.

### Epic I - Decorator/Interceptor Support

Status: [ ] Not started

Target outcomes:

- Add cross-cutting composition without polluting business services.

Planned tasks:

- [ ] Add decorator registration API for service wrapping.
- [ ] Support multiple decorators per service with deterministic order.
- [ ] Add examples for tracing/caching/retry wrappers.
- [ ] Add performance benchmark baseline for decorated resolve path.

Acceptance criteria:

- Decoration does not break current provider resolution guarantees.
- Decorator order is explicit and test-covered.

## Cross-Version Engineering Checklist

- [ ] Keep `cargo test --workspace --all-features` green on each epic.
- [ ] Add one runnable example for every new user-facing feature.
- [ ] Keep mdBook and crate README updated per epic.
- [ ] Maintain backward compatibility unless explicitly marked breaking.

## Risks

- Macro ergonomics vs. compile-time complexity tradeoff.
- Named/multi-binding may increase internal storage complexity.
- Scoped context and lifecycle options can introduce subtle concurrency bugs.

## Last Updated

- 2026-03-05
