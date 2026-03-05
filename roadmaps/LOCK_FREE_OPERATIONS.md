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

- Minimize repeated map lookups in `try_resolve`.
- Cache resolved provider metadata per call path (local variable reuse, not global mutable cache).
- Keep `ResolveGuard` unchanged unless profiling shows it as a bottleneck.

### Phase 4 - Stabilization

- Compare default vs `lock-free` benchmarks.
- Add regression guardrails in CI (benchmark job + threshold report).
- Document when `lock-free` is recommended.

## Baseline Results (2026-03-05)

Command:

- `cargo bench -p fluxdi --bench injector_baseline --features thread-safe -- --sample-size 10 --measurement-time 1 --warm-up-time 1`
- `cargo bench -p fluxdi --bench injector_baseline --features "thread-safe,lock-free" -- --sample-size 10 --measurement-time 1 --warm-up-time 1`

Criterion summary (lower is better):

| Benchmark | `thread-safe` | `thread-safe,lock-free` |
| --- | --- | --- |
| `resolve_cached` | 32.55-33.73 ns | 38.95-40.64 ns |
| `resolve_transient` | 134.94-146.99 ns | 139.66-140.53 ns |
| `resolve_concurrent/2` | 229.78-233.93 us | 232.21-235.94 us |
| `resolve_concurrent/4` | 399.26-403.28 us | 386.93-389.26 us |
| `resolve_concurrent/8` | 698.57-715.08 us | 696.31-704.14 us |
| `resolve_concurrent/16` | 1.3177-1.3311 ms | 1.3182-1.3882 ms |
| `resolve_concurrent/32` | 2.6282-2.6867 ms | 2.1476-2.1694 ms |
| `provider_registration` | 198.43-207.22 ns | 430.83-434.26 ns |

Initial reading:

- `lock-free` helps high-concurrency cached resolve (notably at 32 threads).
- `lock-free` hurts registration cost and single-thread cached resolve.
- Keep `lock-free` opt-in for read-heavy/high-contention paths.

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
- Phase 3+: planned
- Last aligned with codebase: 2026-03-05


