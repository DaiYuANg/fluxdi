# Resource Limits - Injector/Provider Design

This roadmap describes resource-limiting for the current `Injector` + `Provider` architecture.

## Goal

Limit concurrent provider execution per service type to protect expensive dependencies.

## Non-Goals (v1)

- Memory-based limits.
- Automatic live-instance counting.

## Proposed API Shape

New public types:

- `Policy`:
  - `Deny` (fail immediately when limit reached)
  - `Block` (wait synchronously until slot is available)
- `Limits`:
  - `max_concurrent_creations: Option<usize>`
  - `policy: Policy`
  - `timeout: Option<Duration>`

Provider construction helpers (proposed):

- `Provider::root_with_limits(...)`
- `Provider::singleton_with_limits(...)`
- `Provider::transient_with_limits(...)`

## Internal Design

- Add optional limiter state to `Provider<T>`.
- In `Injector::resolve_instance`, acquire limiter permit before executing factory closure.
- Release permit via RAII guard after factory returns.

Thread-safe mode:

- sync resolve: atomic counter + condvar/mutex for `Block` policy (optional timeout).
- async resolve (`resource-limit-async`): `tokio::Semaphore` with optional timeout.

Non-thread-safe mode:

- `Deny` policy first; `Block` can remain unsupported initially.

## Error Surface

Add error kind:

- `ErrorKind::ResourceLimitExceeded`

Add helper:

- `Error::resource_limit_exceeded(type_name, details)`

## Phase Plan

### Phase 1

- Add `Policy`, `Limits`, and internal limiter implementation.
- Add `ResourceLimitExceeded` error kind.
- Wire limiter into provider execution path.

### Phase 2

- Add provider constructors with limits.
- Add docs + example for constrained singleton/transient creation.

### Phase 3

- Optional timeout support for `Block`.
- Optional async integration (`tokio::Semaphore`) behind feature flag.

## Acceptance Criteria

- Limit enforcement works for concurrent resolves.
- Existing resolve semantics are preserved.
- All existing tests pass + new limit tests added.

## Status

- Planning: completed
- Baseline matrix health check: completed on 2026-03-05
- Phase 1: completed (`Policy`/`Limits`, limiter internals, error wiring, injector integration)
- Phase 2: completed (`Provider::*_with_limits` APIs + tests)
- Phase 3: completed baseline (`Limits::with_timeout` + `resource-limit-async` semaphore path)
- Phase 4+: planned
- Last aligned with codebase: 2026-03-05
