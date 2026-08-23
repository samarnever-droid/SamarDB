# 08 — Deadline scheduling: p99.9 as a contract

## The gap

Both documents bound tail latency per mechanism (per-loop bounds, work-credit
currency). Both are bounding the **machine**. Neither bounds the **request**, and
a user does not experience a work-credit budget. A request that hits L1, misses
to flash, and lands on a maintenance-delayed refill can be three orders of
magnitude slower than the 2 µs p99.9 target while every mechanism stays inside
its own bound.

**Compositional bounds on parts do not compose into a bound on the whole when the
whole is a variable-length path.**

## Mechanism

A request carries a latency budget (caller-supplied, or derived from class SLO).
Each tier compares expected cost against remaining slack before it begins:

```
slack := req.budget - elapsed(req.start)
for tier in [L0, L1, L2, L3, L4, ORIGIN]:
    cost := tier.expected_latency_p99(req.class)   # dl[] EWMA
    if cost > slack: return degrade(req, tier)     # ← the whole idea
    ... try_serve ...
```

`degrade` is **not failure** — it is an ordered ladder of things preferable to
blowing the deadline: serve a stale version within declared tolerance → serve a
lower fidelity within declared ε (the type system already guarantees acceptance)
→ serve a partial result → only then fail fast with `DeadlineExceeded`, which is
far better than a timeout at 3× the budget after the origin was consulted.

## Three consequences

1. **Latency becomes contractual** — you cannot miss a deadline you are permitted
   to degrade against; p99.9 becomes an input, not a benchmark result.
2. **Work that can't meet its deadline is cancelled**, reclaiming capacity when
   it's scarce. Caveat: cancelling an in-flight origin query cancels *your*
   waiting, not the origin's work, unless the origin supports cancellation.
3. **Minimise variance, not mean** — "smooth" is a variance property; every prior
   latency mechanism optimises the wrong moment of the distribution.

## Interaction with the price system

Latency slack is simply another priced resource. Tail pressure ⇒ λ_slack rises ⇒
the cache prefers cheaper tiers, lower fidelities, staler versions — same
machinery, no second controller. The SLO `priority: freshness | origin_cost |
latency` field becomes the relative weights on three prices, not a switch
between three hand-written strategies.
