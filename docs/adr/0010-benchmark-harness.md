# ADR 0010 — Performance & Microbenchmark Harness

## Status
Accepted — Phase 9

## Context
To validate SamarDB's low tail-latency claims and ensure continuous performance regression
protection without non-deterministic OS clock dependencies, we require a repeatable benchmark
harness that runs within the deterministic testing framework.

## Decisions

### 1. Deterministic Performance Measurement
- CPU operations, I/O accesses, and buffer hits/misses generate discrete simulated cost units ("ticks").
- A latency sample accumulator records operation durations in a sorted list to compute exact `p50`, `p95`, and `p99` percentiles without approximation errors.

### 2. Workload Suites
- **Storage Layer**: Heap slotted-page allocation, binary search B+ Tree lookups.
- **Relational Layer**: Volcano tuple pipeline iteration, filter selectivity evaluation.
- **Query Optimization Layer**: Selinger DP search space enumeration.

### 3. Metric Assertions
- `p99_latency` must be strictly bounded.
- Point lookup cost must be logarithmic `O(log N)`.
- Buffer pool hit ratio under repeated hot-record access must exceed 90%.

## Consequences
- Performance regression tests can be run as part of standard CI in milliseconds.
- Deterministic results eliminate flaky benchmark runs.
