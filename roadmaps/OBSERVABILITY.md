# Observability - Injector-Centric Plan

This document defines an incremental observability strategy for FluxDI's current architecture.

## Goals

- Add low-overhead tracing around registration and resolution.
- Keep observability fully opt-in via feature flags.
- Keep runtime behavior unchanged when observability features are disabled.

## Architecture Targets

Instrumentation should focus on:

- `Injector::try_provide` (registration)
- `Injector::try_resolve` (resolution)
- provider factory execution path
- circular dependency detection path (`ResolveGuard`)

## Feature Flags

- `tracing` (existing): emit spans/events.
- `opentelemetry` (existing): bridge tracing to OTel via helper wiring.
- `metrics` (existing): internal counters and resolve timing totals.
- `prometheus` (existing): text exposition export for injector metrics.

## Phase Plan

### Phase 1 - Tracing Spans/Events (Now)

- Add spans/events with stable names:
  - `fluxdi.provide`
  - `fluxdi.resolve`
  - `fluxdi.factory.execute`
  - `fluxdi.circular_dependency`
- Add type-name and scope fields when cheap.
- Ensure no-op behavior when no subscriber is installed.

### Phase 2 - Public Observability Module

- Add `fluxdi::observability` module with event/span name constants.
- Keep module lightweight and backend-agnostic.

### Phase 3 - OpenTelemetry Bridge (Optional)

- Add optional `opentelemetry` feature and helper wiring docs.
- Do not auto-initialize global exporters in library code.

### Phase 4 - Metrics (Optional)

- Add counters/histograms for resolve attempts, success/failure, latency.
- Keep labels low cardinality.

## Acceptance Criteria

- Existing tests continue to pass.
- Tracing feature compiles and emits spans/events on key operations.
- No additional dependencies are required for base users.

## Status

- Phase 1: completed (core spans/events implemented and asserted in tests)
- Phase 2: completed (`fluxdi::observability` constants module added)
- Phase 3: completed (`opentelemetry` feature + `fluxdi::observability` helper wiring/docs)
- Phase 4: completed baseline (counters + duration totals + Prometheus text export)
- Phase 5+: planned
- Last aligned with codebase: 2026-03-05


