# 02 — AEGIS reconciliation: retractions & adoptions

AEGIS v7 is a good document written against a bad source: it scored HELIOS
against a nineteen-line feature summary, not the spec, so six of seven "HELIOS
is silent" findings are answered by existing sections (§9, §13, §14). That is a
research error. The design errors are separate and arithmetic.

## The four numbers to retract (each is checkable arithmetic)

1. **"Index-hit GET 12–35 ns" is not a DRAM number.** Accept COMBO ("one cache
   miss, not two") — it is correct — then price it: DRAM bucket load is 70–100 ns
   load-to-use on contemporary DDR4/DDR5, plus ~3–6 ns compute + ~2–4 ns SIMD
   probe. **DRAM-resident ≈ 80–110 ns; 12–35 ns is the L2/L3-resident figure.**
   Publishing it under "index-hit" under-provisions a fleet by ~3×.
2. **"p99 staleness ~35 ms" quotes a percentile of an unbounded tail.** The two
   origin-controlled terms (commit→feed, refill) are unbounded when the feed
   stalls. The honest form is a conditional: `staleness ≤ max(cdc_lag, clamp)`,
   with the lag measured/exported and the clamp enforced by a ladder.
3. **"0 knobs" requires the controller to be infallible.** Shard count, arena
   split, flash over-provisioning, NUMA/hugepages, CDC endpoints, the BYPASS
   capacity — none are derivable from an SLO. A controller that owns every knob
   has no floor. **Hard constraint = token bucket; controller only optimises
   inside the safe region.**
4. **"Within 1.5 % of Belady"** — Belady is optimal only for uniform-size items;
   variable-size offline optimal replacement is NP-hard (knapsack-flavoured), so
   Belady is an approximation of a bound, not a bound. Quoting 0.5 % agreement
   overstates the reference.

## The cuckoo merge is a correctness regression, not a trade-off

Displacement breaks the seqlock's one-bucket identity property: a reader that
validates bucket A alone returns a **false miss** on a key the writer moved to
bucket B. A false miss is a wrong answer (poisons negative caching, single-flight,
and EXACT). **Decision: reject displacement; keep overflow-counter deletion + the
probe bound.** Standing rule: any mechanism that moves an entry between buckets
must be re-validated against the seqlock protocol before adoption.

## What I adopt (with credit), §3

1. **Derived shard count** — withdraw the hardcoded 256. `S = clamp(2^⌈log2 4C⌉,
   16, 2^⌊log2 M/64MiB⌋)`. *Already landed in code* (12-bit field, `shard_count.rs`).
2. **Endurance shadow price** — `admit ⇔ p_reuse·refill_cost / (size·α) > λ` is a
   better *category* (admission is a purchase, λ is a price). Seed of the whole
   price system.
3. **Invalidation-driven write amplification** — flash entries die by write as
   well as capacity; `α` must include invalidation churn. Composes with HELIOS's
   volatility term into one per-dep write rate feeding both DRAM eviction and
   flash admission.
4. **Hedge for the policy tournament** (regret-bounded, replaces greedy leader).
5. **Bloom over the L0 invalidation sequence** (counter first, measure, add Bloom
   only if L0 hit rate suffers).

The through-line: AEGIS's central mechanism — invalidate, then refill from
origin — is the thing MERIDIAN deletes (see `04-delta.md`).
