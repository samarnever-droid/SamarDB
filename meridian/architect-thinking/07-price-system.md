# 07 — The price system: one optimiser, no policies

## State the problem directly

Every admission/eviction decision consumes several scarce resources at once:
DRAM bytes, flash write endurance, origin queries, local CPU, and money/power.
HELIOS §5 chose among policies by tournament; AEGIS §4.5 between two experts by
regret. **Both are searching for a good policy; neither is solving the problem
the policy is a heuristic for.**

Maximise served value subject to resource constraints:

```
max_x  Σ_k v_k x_k
s.t.   Σ_k c_{k,r} x_k ≤ C_r  ∀r ∈ R,   x_k ∈ [0,1]
```

The Lagrangian dual assigns each resource a price λ_r, and the admission rule is
a single inequality:

```
admit k at configuration j  ⇔  v_{k,j} > Σ_r λ_r c_{k,j,r}
```

with prices updated by dual ascent from measured constraint slack:

```
λ_r ← [ λ_r + η (usage_r / C_r − 1) ]⁺
```

A resource under pressure gets expensive and the cache stops spending it —
automatically, with no policy named after anybody.

## Resources and their observable prices

| r | C_r | c_{k,r} | high λ_r means |
|---|---|---|---|
| DRAM bytes | tier capacity | entry+planes at fidelity | demote fidelity, evict, push to flash |
| Flash writes | C·D/(86400·α) | bytes × amplification | raise the flash bar |
| Origin QPS | token-bucket rate | refill prob × cost | prefer maintain=differential, serve stale |
| Local CPU | work-credit budget | delta-apply × λ_w | prefer invalidate over maintain |
| Latency slack | deadline budget | expected tier latency | skip tiers, serve approximate |
| Money/power | $/hr, watts | tier-proportional | shrink DRAM, lean on flash |

## Every policy in both documents is a corner of this space

Set the prices to fixed values and the general rule degenerates exactly: LFU, LRU,
GDSF, LeCaR, W-TinyLFU, the ATLAS tournament, and AEGIS §2.1's flash inequality
are all corners. The tournament worked because it was sampling corners; it
plateaus because the optimum is usually **not** at a corner, and no amount of
switching between corners reaches an interior point.

## Where the machinery goes

- **Hedge** estimates `v_k` (reuse + cost prediction) — the genuinely uncertain
  part that benefits from a regret bound.
- **Dual ascent** handles allocation — constrained, not uncertain.

Prediction where you're uncertain, optimisation where you're constrained. The
tournament conflated the two; so did LeCaR.

## Guardrails (control loops misbehave)

η small + EWMA-smoothed (slower than workload phase changes); hard ceiling on
every λ_r (degrade gracefully, never admit nothing); per-class floors as hard
constraints outside the optimisation; and — restating §2.3 — **the origin token
bucket is not a price**. Prices are how the cache optimises inside the safe
region; buckets are what make the region safe. A price can be wrong; a bucket
cannot exceed its rate.
