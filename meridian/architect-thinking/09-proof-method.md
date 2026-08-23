# 09 — Proof, not a proof harness

AEGIS puts the proof harness at W24 (last), verifying a protocol built at W3 —
which is how you discover at W24 that W3 was wrong, with 21 weeks on top of it.
HELIOS §15 puts the simulator in Phase 0, before implementation. The ordering
difference is not a preference.

Three levels, in increasing order of what they establish:

## 1. The step bound becomes a type

The audit table ("every hot loop is bounded, checked by a human, once") becomes a
compiler obligation:

```rust
#[bounded(64)]                      // ≤ 64 steps, or it does not compile
fn probe(b: &Bucket, tag: u8) -> Option<u8> { ... }

#[bounded(1024)]
fn drain_pending(q: &mut Queue) -> u32 {
    while !q.is_empty() { @decreases(q.len()) ... }   // explicit measure required
}
```

Deliberately shallow: constant trip counts, runtime bounds with a proven ceiling,
or an explicit `@decreases` measure. Far less than the full borrow checker, but it
converts "we audited the loops" into "an unbounded hot-path loop fails the build."

## 2. The concurrency protocol gets a machine-checked model

Seqlock + epoch was already subtle enough that shipping only one half is a
one-in-a-billion wrong-answer bug class. Adding version chains + snapshot
watermarks makes prose reasoning indefensible. A TLA+ spec covering reader
validation, displacement-free mutation, epoch advance, version append, and
watermark GC — model-checked for: no torn value, no reclaimed cell, no snapshot
observes a version outside its watermark, no version reclaimed while reachable —
with a refinement harness driven by the same trace generator as the simulator.

## 3. The simulator is the reference implementation, not a test

A deterministic, single-threaded, seeded model of the entire policy + freshness
stack (admission, eviction, fidelity, prices, maintenance plans, CDC, ladder
transitions) that consumes a trace and emits a decision log. It exists **before**
the fast implementation, decides policy questions before they cost engineering
time, and the fast implementation must produce a **byte-identical decision log**
on the same trace and seed. Production incidents are captured as trace + seed and
replayed deterministically. That is the difference between "we tested it" and
"we can reproduce it."

## The falsification duty

Every number in the target table carries a falsifier, and each phase's acceptance
gate is that someone **actively tried to trip the falsifier and failed**.
