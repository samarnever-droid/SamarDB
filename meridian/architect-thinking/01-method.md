# 01 — Basis & Falsifier (the method)

Every quantitative table carries three columns:

- **Value** — the number.
- **Basis** — `derived` (follows from a stated model — check the arithmetic),
  `measured` (an observation, hardware named), `target` (aspiration with owner
  + gate), `bound` (a worst case a mechanism enforces).
- **Falsifier** — the observation that would prove the number wrong.

## Why this is the entire method

A number without a basis is a claim. A number with a basis is an argument. And
an argument you cannot name the falsifier for is not an argument you have
tested — it is an argument you have only asserted.

The errors found in §2 (AEGIS's 12–35 ns DRAM figure, the 35 ms p99 staleness,
"0 knobs", "within 1.5 % of Belady") were **only findable** because the basis
column was added. Each is arithmetic or mechanism, not taste.

## The honest consequence

I found three of my own errors this way while writing the design:

1. An early section claimed 70–95 ns for a random GET; the bucket → entry →
   cell chain is dependent and serialises at ~80 ns/hop, so ~240 ns. **The
   correction produced §3.10's dual arena** — the mechanism came from the
   corrected number, not the other way around.
2. The 16 B entry kept refusing to grow (provenance, generations pushed to side
   arrays) — **that produced §4.2's side-planes**.
3. (a third, unspecified, resolved mid-draft)

**Rule:** causality must run from the honest number to the mechanism, never the
reverse. If a mechanism was chosen first and the number came after, the number
is decoration.

## Carrying it forward

- Every benchmark row in `benchmarks/results.jsonl` should grow a `falsifier` tag.
- Every phase gate in `TODO.md` is a falsifier someone must actively try to trip.
- A benchmark suite proves the system works on the cases you thought of. The
  falsifier column is the list of cases that would prove it doesn't — and
  running those is the only part of measurement that can change your mind.
