# 03 — Capability side-planes (the structural fix)

## The local optimum

The 16 B entry (`key_hash32 | cell_ref u48 | ctl u16 | wheel_cookie u16 |
cost_log u8 | freq u8`, four-per-line, zero spare bits) is a packing
achievement — and a structure that has **stopped**. Every later need got pushed
to a side array as a "reluctant compromise":

- provenance → `prov_ref[slot]` (+4 B)
- posting generations → `reuse_gen[slot]` (+1 B)

Those weren't compromises; they were the design telling me what it wanted to be.
AEGIS §5.1 found the third instance (COMBO's 64 B constraint forces dep sets out
of line) and correctly read it as an improvement, not a concession — then stopped
one step short of the generalisation.

## The generalisation

MERIDIAN needs four more optional things per entry: versions, fidelity,
maintenance state, deadline stats. Inline is hopeless. So:

- Entry keeps 16 B, gains a **3-bit `planes` field**, funded honestly from `ctl`:
  narrow `FREQ_SNAP` 4→2 (the admission gate only compares against a threshold)
  and reclaim the `PINNED` bit into the size-class encoding. **No growth, no new
  cache line, four-per-line preserved.**
- Each capability is a **parallel per-shard array** indexed by the same `slot_idx`
  — addressing is free (the index is already in a register).

| Plane | B/entry | On read path? |
|---|---|---|
| `prov[]` dep-set ref + created_lsn | 8 | No (miss-fill/eviction only) |
| `gen[]` posting reuse generation | 1 | No |
| `vers[]` version chain + watermark | 11 | Only for snapshot reads |
| `fid[]` fidelity + error bound | 2 | No (admission) |
| `maint[]` differential operator state | 6 | No (invalidation only) |
| `dl[]` latency EWMA + slack | 4 | No (recorded after completion) |

## Why it works

**Plane arrays are cold.** Dense, sequentially scanned by maintenance, never on
the hot read path. Putting them inline would make the hot path pay for
capabilities most entries don't use. Beside it, an entry with no provenance/
versions/fidelity costs exactly what it cost in v4 — 16 B and one cache line —
while its neighbour carries all six planes.

Maximal configuration: 16 + 32 = 48 B/entry vs v4's 25 B (47 B with ORACLE) —
the full feature set costs ~1 B/entry more than v4-with-ORACLE, and everything
above `gen[]` is per-class or per-call-site opt-in.

I would not have found this without AEGIS §5.1 forcing the constraint into the
open. That is what a good adversary does to a design.
