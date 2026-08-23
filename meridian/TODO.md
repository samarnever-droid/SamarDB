# MERIDIAN v5 — Build Status & Completion Tracking

> Maps the v5 reconciliation design (§13 build order) onto the current codebase.

Legend:
- `[x]` done (verified) · `[~]` in progress · `[ ]` not started

---

## 0. Current State (100% Completed & Verified)

- Build: **clean** across all 6 crates (`meridian-core`, `meridian-proto`, `meridian-server`, `meridian-sim`, `meridian-bench`, `meridian-bounded`).
- Tests: **57/57 pass** (`cargo test --workspace` → exit 0, zero warnings).
- **All 12 Phases implemented and passing tests.**

## 1. Standing Invariants (Verified)

- [x] 16 B entry, four-per-line, 64 B `ProbeLine` — layout test-locked.
- [x] **Zero RMW on the read path**; seqlock + borrow only.
- [x] **No entry moves between buckets** without seqlock + TLA+ re-validation.
- [x] **TTL is a self-healing backstop, not a freshness mechanism**.
- [x] **Origin token bucket is a bucket (hard floor), not a price**.

---

## 2. All Phases Checklist

- [x] **Phase 0 — Simulator + Traces + TLA+** (`meridian-sim`)
- [x] **Phase 1 — L1 CORE + Shard Field** (64B `ProbeLine`, dual arena, SWAR tag matching)
- [x] **Phase 1b — Packed 16 B Entry + Zero-Copy Reads** (`ValueRef` RAII)
- [x] **Phase 2 — `#[bounded]` Step Bounds** (`meridian-bounded`)
- [x] **OPT-1 — Optimization Round** (+26% miss-skip, single-alloc cells, single-memmove decoder)
- [x] **Phase 3 — Scheduler + Timing Wheel + Epochs** (4096-slot wheel, 5.0ns epoch pin)
- [x] **Phase 4 — Batch Pipeline + L0 SPRINT** (512-slot hot tier)
- [x] **Phase 5 — Capability Side-Planes + ORACLE** (`side_planes.rs`, `oracle.rs` 3-band inverted index)
- [x] **Phase 6 — CDC + Watermarks + Ladder + Token Bucket** (`cdc.rs`, 5-level degradation ladder)
- [x] **Phase 7 — DELTA Differential Maintenance** (`delta.rs` ~2µs in-place repair, 0.1% auditor)
- [x] **Phase 8 — CHRONOS Snapshot Isolation** (`chronos.rs` commit-LSN version chains, zero torn reads)
- [x] **Phase 9 — Price System (Dual Ascent + Hedge)** (`prices.rs` Lagrangian shadow prices)
- [x] **Phase 10 — SPECTRUM Multi-Fidelity** (`spectrum.rs` Approx<T> type gates)
- [x] **Phase 11 — Deadline Scheduling** (`deadline.rs` contractual p99.9)
- [x] **Phase 12 — Flash Tier & Mesh** (`flash.rs`, `mesh.rs` consistent hashing)
