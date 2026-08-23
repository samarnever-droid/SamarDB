# MERIDIAN

A Rust implementation of the **HELIOS v5 — MERIDIAN** cache specification:
a sharded, seqlock-read cache engine with a Redis-compatible wire protocol and
a native `MD.*` API, plus the Phase 0 deterministic policy simulator.

**Status: research prototype.** Phases 0–1 vertical slice. Not production.
See [PHASES.md](PHASES.md) for the spec build order and every deliberate
deviation from it.

## Workspace layout

| Crate | Role | Spec |
| --- | --- | --- |
| `meridian-sim` | Deterministic trace-driven simulator: FIFO, LRU, Clock, Belady; seeded Zipf traces; decision digest for run-to-run identity | §13 Phase 0 |
| `meridian-core` | The engine: derived shard count, one-line probe reads (zero RMW), bounded probe window, TTL timing wheel, eviction, epoch reclamation | §3.1, §3.2, §3.3 |
| `meridian-proto` | RESP2/RESP3 decode + encode | Redis compatibility |
| `meridian-server` | Thread-per-connection TCP server: Redis command subset + `MD.*` native API | product surface |

## Quickstart

```sh
cargo run --release -p meridian-server            # listens on 127.0.0.1:7717
cargo run --release -p meridian-server -- --port 6379 --cores 8
```

Then point any Redis client at it:

```sh
redis-cli -p 7717 ping                     # PONG
redis-cli -p 7717 set session:42 abc EX 60 # OK
redis-cli -p 7717 get session:42           # "abc"
redis-cli -p 7717 ttl session:42           # 60..1
```

Options: `--host`, `--port`, `--shards` (override the derived count),
`--cores` (pretend N cores for shard derivation), `--entries` (capacity hint),
`--min-buckets` (floor on buckets per shard — set to 1 with `--shards 1
--entries 14` for a capacity-of-exactly-14 test instance).

Pipelining is supported: a batch of N commands arriving in one TCP segment is
answered with one reply write per batch.

## Benchmarks

```sh
cargo run --release -p meridian-bench -- --phase 0-1 --secs 2
```

Phase-tagged suite (engine in-process, TCP server, simulator). Every run
appends to `benchmarks/results.jsonl` so numbers accumulate across phases;
the per-phase contract and current baselines live in
[BENCHMARKS.md](BENCHMARKS.md).

## Load generator

```sh
cargo run --release -p meridian-server --example loadgen -- 127.0.0.1:7717 200000 10000
```

Reports pipelined SET/GET throughput and sequential PING RTT over one
connection. Indicative loopback numbers on the dev machine (Windows, single
connection, pipeline ×128): ~0.3–0.8 M ops/s GET, ~6–15 µs PING RTT. These
measure the TCP path, not the spec §12 in-process engine latencies.

## Commands

Redis subset: `PING ECHO HELLO COMMAND CONFIG CLIENT QUIT GET SET DEL EXISTS
EXPIRE TTL PTTL DBSIZE FLUSHALL FLUSHDB INFO MGET MSET SCAN`.

`SET` supports `EX`, `PX`, `NX`, `XX`, `KEEPTTL`, `GET`.
`HELLO 3` switches the connection to RESP3 (maps, `_` nulls).

Native API (the surface that will grow into §13.10 and later phases):

```
MD.STATS                                  # shards/items/hits/misses/hit_ratio/...
MD.SLO SET dashboard freshness_p99_ms=250 origin_qps_max=2000 priority=2
MD.SLO GET dashboard | MD.SLO DEL dashboard | MD.SLO LIST
```

SLO declarations are stored and echoed today; the controller that consumes
them (TTL multipliers, refill limits) is Phase 6+.

## Simulator

```sh
cargo run --release -p meridian-sim
```

Prints hit-ratio tables for FIFO/LRU/Clock/Belady over uniform and Zipf
traces. Same seed ⇒ identical decision digest; the fast engine will later be
required to match the simulator's decision log on the same trace (§10).

## Tests

```sh
cargo test --workspace
```

Includes a seqlock stress test: concurrent writers flip one key between two
64-byte values while readers assert they only ever observe one of the two —
never a torn mixture.

## Honest deviations (v0)

- Entry is 24 B (atomic fields for a sound seqlock), not the packed 16 B u48
  COMBO line — Phase 1 gate.
- Scalar way scan, not 14-way SIMD tag probe — Phase 1.
- Reclamation is epoch-based (5 ns pin/unpair): `ValueRef` holds its pin,
  so a zero-copy handle keeps its cell alive for as long as it lives
  (`!Send` — release on the owning thread).
- Blocking thread-per-connection server, not an event loop with batching —
  Phase 4.
- Tables are sized at startup; no incremental resize — Phase 3.

Full list and gate status: [PHASES.md](PHASES.md).
