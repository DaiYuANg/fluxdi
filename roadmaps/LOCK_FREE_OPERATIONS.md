# Lock-Free Operations - Injector-Centric Plan

This roadmap tracks a lock-contention reduction effort for the current FluxDI architecture (`Injector`, `Provider`, `Application`).

## Scope

- Keep public APIs stable.
- Keep `thread-safe` behavior correct first, optimize second.
- Gate risky optimizations behind explicit feature flags.

## Current Lock Points

In `thread-safe` mode, `Injector` currently keeps:

- `providers: RwLock<HashMap<TypeId, Shared<dyn Any + Send + Sync>>>`
- `instances: RwLock<HashMap<TypeId, Shared<dyn Any + Send + Sync>>>`

Hot paths:

1. `try_resolve` reads `instances`.
2. Cache-miss path reads `providers` and may write `instances`.
3. Registration writes `providers`.

Circular detection is already thread-local (`ResolveGuard`) and is not a major contention point.

## Design Principles

1. Preserve semantics of `Scope::{Transient, Module, Root}`.
2. Optimize read-heavy paths first.
3. Keep `lock-free` optional until benchmarked.
4. No unsafe code unless there is a measured need.

## Proposed Phases

### Phase 1 - Benchmark Baseline

- Add `criterion` benches for:
  - cached resolve
  - transient resolve
  - concurrent resolve (2/4/8/16/32 threads)
  - provider registration
- Publish baseline numbers in this file.

### Phase 2 - Optional Concurrent Maps (`lock-free` feature)

- Add optional dependency: `dashmap`.
- Add feature: `lock-free = ["thread-safe", "dep:dashmap"]`.
- In `thread-safe + lock-free` mode, use `DashMap` for provider/instance stores.
- Keep current `RwLock<HashMap<...>>` as default implementation.

### Phase 3 - Reduce Resolve Path Overhead

- [x] Fast-path optimization: skip `ResolveGuard::push` on cache hit (no recursion, no circular dep).
- Minimize repeated map lookups in `try_resolve` (cache-miss path).
- Cache resolved provider metadata per call path (local variable reuse, not global mutable cache).

### Phase 4 - Stabilization

- Compare default vs `lock-free` benchmarks.
- Add regression guardrails in CI (benchmark job + threshold report).
- Document when `lock-free` is recommended.

## Baseline Results

**2026-03-21** (Windows, release, criterion default):

| Benchmark | default | thread-safe | lock-free |
| --- | --- | --- | --- |
| `resolve_cached` | ~12 ns | ~36 ns | ~35 ns |
| `resolve_transient` | ~102 ns | ~117 ns | ~147 ns |
| `provider_registration` | ~152 ns | ~327 ns | ~3.8 µs |
| `resolve_concurrent/2` | — | ~126 µs | ~178 µs |
| `resolve_concurrent/8` | — | ~431 µs | ~334 µs ✓ |
| `resolve_concurrent/32` | — | ~1.34 ms | ~1.23 ms ✓ |

Command:

- `cargo bench -p fluxdi`
- `cargo bench -p fluxdi --features thread-safe`
- `cargo bench -p fluxdi --features "thread-safe,lock-free"`

Summary:

- Default (Rc) is fastest for single-thread.
- `lock-free` helps high-concurrency cached resolve (8+ threads).
- `lock-free` significantly hurts provider registration; use only when registration is rare.

## Acceptance Criteria

- No behavior changes in existing tests.
- `cargo test -p fluxdi` passes for:
  - default features
  - `--features thread-safe`
  - `--features "thread-safe,lock-free"`
- Concurrent resolve throughput improves measurably in synthetic benchmarks.

## Risks

- Added dependency (`dashmap`) and larger binary size.
- Different contention profile under low-core machines.

Mitigation: keep optimization optional and benchmark-driven.

## Status

- Planning: completed
- Baseline matrix health check: completed on 2026-03-05
- Phase 1 (benchmark baseline): completed on 2026-03-05
- Phase 2 (`lock-free` feature + `DashMap` stores): completed on 2026-03-05
- Phase 3 (partial): cache-hit fast path implemented
- Last aligned with codebase: 2026-03-05


