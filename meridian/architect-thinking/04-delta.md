# 04 — DELTA: repair the value, do not discard it

## The move

For cached results with algebraic structure, apply the change delta to the
cached value **in place**. Origin QPS for those entries goes to **zero** — not
coalesced, not rate-limited, not bucketed. Zero, because no request is made.

## Why the delta was already the answer

An invalidation under ORACLE says: *this entry was derived from `Dep::Row(users,
4021)`, and here is the new value of that row.* The delta is already in the CDC
stream, decoded, in L1 on the owning shard. v4 read it, located the entry, then
threw the delta away and refilled from origin. v7 did the same and billed for it
more carefully. **The delta was the answer.**

## The arithmetic (canonical dashboard example)

λ_r = 10/s reads, λ_w = 2/s writes, 20-row indexed refill ≈ 1 ms origin CPU:

| | Invalidate + refill | Differential repair |
|---|---|---|
| Origin CPU | 2 ms/s | 0 |
| Local CPU | ~0 | 4 µs/s (~2 µs/delta) |
| 10 k entries | 20 saturated origin cores | 0.04 local cores |
| Write burst | origin melts (∝ min(λ_w,λ_r)) | local CPU absorbed by work-credit scheduler |
| Staleness | cdc + refill (on user critical path) | cdc + apply (~2 µs, off path) |

**Stampede protection becomes vacuous** on this path — single-flight, XFetch,
leases, negative caching, the origin token bucket, load-shedding (≈ a quarter of
both documents by mechanism count) exist to manage the cost of asking again.
There is no thundering herd for a value that was never evicted and never went
stale. They stay in the build for the honest remainder (cold start, non-repairable
tail, capacity eviction, > MAX_FANOUT lazy band, PROVENANCE_ONLY and below).

## What is repairable (and the compiler knows which)

| Class | Operators | Cost |
|---|---|---|
| Linear | select/project/union/count/sum/monoid group-by | O(|Δ|) |
| Indexed join | keyed + resident side | O(|Δ|·f) |
| Bounded-state | top-K / windowed / limit-after-order | O(|Δ| log K) + K′ buffer |
| Semilattice | min/max/distinct | O(|Δ|) … O(log n) with deletes |
| **Not repairable** | exact median/percentile, count-distinct, recursion, opaque code | — fallback to invalidate |

The classification is decidable at compile time from the same MIR pass that
already types `#[cached]` bodies. `maintain = differential` on a non-repairable
body is a **compile error naming the operator and line**. The developer never
writes a delta rule; they annotate intent and the compiler proves it or refuses.

## The correctness obligation (stated as an obligation)

Three defences, in strength order:

1. **Algebraic** — delta rules are generated from a fixed per-operator library,
   each proven once; composition of correct incremental operators is correct by
   induction. Proof burden is O(operators), paid once.
2. **Continuous differential audit** — 0.1 % of maintained entries recomputed from
   origin in the background, byte-compared. Mismatch ⇒ hard integrity fault:
   drop to `maintain = invalidate`, flush the plan, alarm, record the delta
   sequence for replay.
3. **TTL, still** — a maintenance plan is a richer thing to be wrong about than a
   dep set; the bounded TTL is the backstop that heals what the other two miss.

The audit is what makes DELTA shippable rather than merely elegant: the system
does not trust its own arithmetic — it audits it, continuously, in production.
