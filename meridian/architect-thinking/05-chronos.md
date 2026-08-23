# 05 — CHRONOS: closing the torn-read hole

## The defect (disclosed, addressed by neither AEGIS nor v6/v7)

Two entries invalidated by one origin transaction may refresh at different
times, so a reader can observe a torn view across two cache entries. This is
**invisible to every metric both documents propose**: staleness p99 says nothing
about it; EXACT staleness ≤ cdc_lag does not prevent it (both entries can be
individually within bound yet mutually inconsistent). A page rendering a balance
from one entry and a transaction list from another can show a transfer debited
but not credited — each entry correct, the page wrong, no alarm.

**Exactness per entry does not compose into consistency across entries, and
provenance cannot fix it, because provenance is a per-entry property.**

## Mechanism

Every origin write carries a commit LSN (already in the CDC stream, already used
for replay suppression). Promote it to a global snapshot watermark + version chain:

```
@repr(exact)
struct Version              # vers[] plane, 11 B
    valid_from: u48         # commit LSN at which this version became current
    payload:    u40         # cell_ref into the arena
    next:       u24         # prior version, slot-local chain, 0 = end
```

A snapshot read pins watermark `W` and, per entry, walks the chain to the newest
version with `valid_from ≤ W`. A read set opened at one watermark and served
entirely from it is **transactionally consistent** across every entry — a
guarantee neither prior document offers at any staleness bound.

Watermark pinning **reuses the epoch machinery verbatim**: an open snapshot is an
epoch participant; version GC is epoch reclamation with the predicate changed
from "no reader can see this" to "no snapshot can see this". No new protocol, no
new failure mode, and the seqlock interaction is unchanged because **versions are
appended, never moved** — the exact property that makes this compose and the
cuckoo displacement of §2.5 not.

## Why it is affordable

You only need versions inside the snapshot retention window, not the entry's
lifetime. Mean versions = `1 + λ_w · W`:

| Class | λ_w | W | Mean versions | Overhead |
|---|---|---|---|---|
| Reference data | 0.001/s | 2 s | 1.002 | negligible |
| Typical entity | 0.1/s | 2 s | 1.2 | +2.2 B |
| Hot dashboard | 2/s | 2 s | 5 | +44 B — **do not version** |
| Hot dashboard | 2/s | 100 ms | 1.2 | +2.2 B |

A short window is a *shorter-lived* guarantee, not a weaker one. An overrun fails
cleanly with `SnapshotExpired` (the MVCC discipline for long-running transactions).

## What it does not give

Snapshot isolation over the origin's commit order — **not** serializability, **not**
linearizability. A snapshot may be arbitrarily stale (consistent, not fresh —
independent axes). No write-your-own-reads: a client must pin the watermark its
write returned.
