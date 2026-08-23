# 10 — What MERIDIAN does not do

A design's honesty is measured by what it declines. This list matters more than
the feature list.

- **It is not a database.** It maintains derived values it was told how to
  derive; it does not accept writes, does not order transactions, is not a source
  of truth.
- **It is not serializable or linearizable.** §6 gives snapshot isolation over the
  origin's commit order, which is weaker; read-your-own-writes requires the client
  to pin the watermark its write returned.
- **Differential maintenance does not cover arbitrary code.** Opaque bodies,
  exact percentiles, count-distinct, and non-monotone recursion fall back to
  invalidation. The compiler will tell you which, at the line — but it will tell
  you **no**.
- **It does not do full incremental view maintenance.** The operator library is a
  fixed, proven set. A user cannot supply a custom delta rule — a custom delta
  rule is an unproven correctness claim in the most dangerous position in the
  system. Extending the library is a change to the cache, reviewed as such.
- **Fidelity reduction applies only where a consumer declared tolerance** — never
  to money, identity, authorisation, or audit. The type system enforces this,
  which is the only version of that promise worth making.
- **Prices can be wrong.** Dual ascent is a control loop with ceilings, smoothing,
  per-class floors, and a token bucket underneath. The bucket protects the origin;
  prices only decide how well the cache uses the room the bucket leaves.

## The honest concession

On the pure engine, the two designs have converged: COMBO packing, SIMD probing,
overflow-counter deletion, zero-RMW reads, work credits, incremental resize,
endurance governance — after §3's adoptions there is no meaningful daylight
between v5 and v7 on how fast one box serves a hit. **That question is closed,
and closing it was worth doing. It is simply no longer the question.**
