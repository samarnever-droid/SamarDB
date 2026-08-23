# BENCHMARKS — phase-tagged suite and registry

Run (release only — debug numbers are meaningless):

```sh
cargo run --release -p meridian-bench -- --phase 0-1 --secs 2
cargo run --release -p meridian-bench -- --suite engine|server|sim
```

Every run appends one JSON line per metric to `benchmarks/results.jsonl`,
tagged with `phase`, timestamp and core count — numbers accumulate across
phases, so any future run can be diffed against history:

```sh
grep '"phase":"1"' benchmarks/results.jsonl   # compare phases
```

Full suite takes ~40–60 s (the 2 M-key DRAM working set dominates; use
`--secs 1` for a quick pass). All sockets carry 10 s read/write timeouts, so
a framing bug fails loudly instead of hanging.

## Baseline — phase 0-1 (2026-08-22, 8 cores, Windows, release)

| Bench | Metric | Value | Note |
| --- | --- | --- | --- |
| engine/get_hit_llc | ops/s | ~2.3 M | 8 k-key working set; earlier same-build run hit 5.1 M — run-to-run variance on this box is large |
| engine/get_hit_dram | ops/s | ~2.4 M | 2 M-key working set; earlier run 1.4 M — treat the LLC/DRAM pair as order-of-magnitude until Phase 1 measurement rig |
| engine/set_replace | ops/s | ~2.0 M | alloc + publish + retire path |
| engine/ttl_read | ops/s | ~2.3 M | expire-check on read |
| engine/get_hit_4r1w | ops/s | ~8.7 M | 4 readers + 1 writer, combined; seqlock retries ≈ 0 |
| engine/sweep | sweeps/s | ~18 k | budgeted maintenance scan |
| engine/hash_key | ops/s | ~42 M | the floor under every op |
| server/ping_rtt | p50/p99/p99.9 | 18 / 67 / 240 µs | sequential RTT, loopback |
| server/pipeline_set | ops/s | ~0.7 M | pipeline ×128, 1 connection |
| server/pipeline_get | ops/s | ~0.9 M | all hits |
| sim/policy_lru | ops/s, ratio | 13.5 M, 0.80 | 200 k-op Zipf trace |
| sim/policy_belady | ops/s, ratio | 0.5 M, 0.90 | offline reference |

Variance warning: this dev machine is noisy (background load, thermals).
For decisions, run the suite 3× and compare medians; the JSONL registry
exists precisely so this is mechanical.

## Baseline — phase 1 (2026-08-22, 8 cores, Windows, release)

Tag-probed single-line bucket reads landed (spec §3.2's 14-way SIMD tag probe
in SWAR form: version + 16 tag bytes = one cache line; entry lines touched
only on a tag match). Comparison against the phase 0-1 rows above:

| Bench | Phase 0-1 | Phase 1 | Delta |
| --- | --- | --- | --- |
| engine/get_hit_llc | ~2.3 M ops/s | **7.0 M ops/s** | **3.1×** |
| engine/get_hit_dram | ~2.4 M ops/s | **3.4 M ops/s** | **1.4×** |
| engine/set_replace | ~2.0 M | 2.5 M | +22 % (carries one extra tag RMW) |
| engine/ttl_read | ~2.3 M | 3.6 M | +56 % |
| engine/get_hit_4r1w | ~8.7 M | 12.0 M | +38 % |
| engine/sweep | ~18 k | 27 k | +53 % |
| engine/tag_match | — | 750 M matches/s | new |
| engine/lookup_miss_dram | — | 2.5 M ops/s | new: the number the packed COMBO layout must move |
| server/ping_rtt p99.9 | 240 µs | 96 µs | fewer cold lines per probe |
| server/pipeline_get | 0.9 M | 1.1 M | +22 % |
| server/pipeline_set | 0.7 M | 0.63 M | −10 % (tag RMW on the write path; watch it) |

Still owed for the phase 1 gate: the packed 16 B u48 COMBO entry (kills the
value-clone and the residual entry-line touch), true `pcmpeqb/movmskb`
intrinsics once atomics leave the layout, and perf-counter residency
verification of the LLC/DRAM pair — until then these numbers are
working-set-derived, not hardware-verified.

### Phase 1, second run — 16 B entry + zero-copy reads

The entry is back to the spec §3.5 16-byte size (key_hash moved into the tag
line; `cell@0, ctl@8, wheel@10, cost@12, freq@13`), and `get_ref` returns a
`ValueRef` that skips the per-get allocation (grace-window contract,
documented at the type).

| Bench | Run 1 | Run 2 | Delta |
| --- | --- | --- | --- |
| engine/get_hit_dram (clone) | 3.4 M | 3.2 M | noise |
| engine/get_hit_dram_ref (zero-copy) | — | **4.4 M ops/s** | **+31 % vs clone** |
| engine/get_hit_llc (clone) | 7.0 M | 7.9 M | noise-range |
| engine/set_replace | 2.5 M | 3.7 M | **+48 %** (smaller entries on the write path) |
| engine/ttl_read | 3.6 M | 5.1 M | +42 % |
| server/pipeline_* (in-process suite) | 0.63 / 1.1 M | 0.41–0.50 M | see note |

**Open investigation (recorded, not hidden):** the bench's in-process server
suite regressed vs run 1, but cross-process `loadgen` against a standalone
server shows GET 1.06–1.08 M ops/s and SET 0.73–0.83 M — the server build did
not regress. Suspect: allocator/page-state pollution inside the bench process
after ~1 GB of engine-bench churn. Action: run the server suite in a child
process (or `--suite server` on a cold process) before treating its numbers
as comparable; tracked as follow-up.



## Baseline — phase 2 (2026-08-22, 8 cores, Windows, release)

Phase 2 is a compile gate, so the deliverables are machine checks, not ops/s:

- **Structure gate**: bare `loop`, `while`, and non-range `for` are compile
  errors inside `#[bounded(n)]` functions — verified by negative-path check
  (three violations → three precise errors).
- **Numeric teeth**: the deepest loop nest with literal or const trip counts
  gets a compile-time assertion against `n` using the real constant values —
  verified by negative-path check: `#[bounded(64)]` + `for _ in 0..128`
  (literal) and + `for _ in 0..N` with `const N = 128` both fail the build,
  the latter as `assertion failed: (N) <= 64` at const-eval. Live bindings:
  `lookup` asserts `PROBE_LIMIT * SEQLOCK_RETRIES * WAYS <= 3584`,
  `set_opts`/`locked_find` assert `PROBE_LIMIT * WAYS <= 56`.
- **Coverage**: a test (`gate_coverage.rs`) fails if any hot-path function
  (`lookup`, `set_opts`, `get`, `get_ref`, `locked_find`, `locked_lookup`,
  `drain_retired`) loses its `#[bounded]` attribute.

Runtime numbers are neutral-to-better within the machine's noise band:
get_hit_llc 7.8 M, get_hit_dram 3.6 M / 3.7 M zero-copy, set_replace 3.9 M,
lookup_miss_dram 2.57 M (best yet), ping p99.9 75 µs, pipeline_set 0.81 M /
pipeline_get 1.12 M (the in-process server suite recovered from the phase-1
dip — supporting the allocator-pollution theory; follow-up stands).

Semantics note: the numeric bound covers the deepest single nest; sequential
sibling nests and runtime snapshot loops (`0..len`) are structure-checked
only — the nightly rustc-driver MIR pass (deferred 2b) closes both.

## Baseline — phase 3 (2026-08-22, 8 cores, Windows, release)

Timing wheel landed, replacing the scan-everything sweeper:

- `engine/sweep`: **418 k ticks/s idle** (was 24–27 k sweeps/s walking the
  table) — maintenance cost is now independent of table size.
- `engine/wheel_expire`: **207 k expirations/s** draining 100 k staggered
  deadlines (bound by the stagger span, not the mechanism).
- New wheel tests: staggered deadlines expire on schedule (wide margins),
  overwrite/delete leave inert descriptors (no wrong unlink, no phantom
  expiry), full drain to zero.

Two real bugs found and fixed while building it, both worth remembering:

1. **Floor-indexed slots**: pushing at floor(deadline/tick) let the cursor
   arrive up to 31 ms early; the not-yet-due reinsert then waited a full
   131 s cycle. Fix: ceil-indexing (≤ 1 tick late, the standard
   approximate-wheel jitter).
2. **Runaway cursor**: a `.max(start + 1)` "progress guarantee" forced the
   cursor one tick forward per sweep call; called every ~2.5 ms against a
   32 ms tick, it sprinted ~12× faster than the clock, lapped the 4096-slot
   ring in ~10 s, and delayed every pending expiry by a full cycle (100 k
   deadlines drained in 10.2 s instead of 0.23 s). Fix: the cursor may
   never pass the clock — faster sweep calls are no-ops. Lesson: never add
   forward progress to a clock-driven cursor.

Known gap: burst size within a single slot is unbounded until the
work-credit scheduler lands (a slot holding 100 k deadlines unlinks them in
one sweep call).

### Phase 3, second run — epoch reclamation

The 500 ms time-grace is gone: readers pin via plain stores to thread-owned
registry slots (no RMW, no cross-thread contention), writers tag retires
with the global epoch, and the sweeper frees garbage below the collector
barrier. Measured pin/unpair cost: **5.0 ns**; `lookup_miss_dram` reached
**4.68 M ops/s** in the same run — the best yet, i.e. safety cost nothing.
`ValueRef` holds its pin for its lifetime (RAII): a held handle provably
blocks reclamation of its cell — verified by
`epoch_reclamation_waits_for_pinned_refs` (50 sweeps with the ref held →
zero frees; drop → freed). Run-to-run machine variance still dwarfs
per-change deltas on the hit path; the calibration table above is the lens.

## OPT-1 — optimization round after phases 0–3 (2026-08-22)

New cadence rule (PHASES.md): **after every three phases, an OPT-n round** —
profile-driven changes, each carrying a claimed mechanism; > 10 % regressions
block; results append to the registry under the `opt-n` tag.

Changes and their measured verdicts:

| Change | Mechanism | Verdict |
| --- | --- | --- |
| Miss-skip on the home bucket's overflow counter (§3.3 paying for reads) | a no-spill home bucket ends the probe after one line load instead of up to four dependent DRAM loads | `lookup_miss_dram` 2.57 M → **3.23 M ops/s (+26 %)** — the one consistent win |
| Single-allocation cells (key+value in one `Box`) | 3 allocs → 1 per SET; key compare and value land in the same lines | structural; set_replace moved 1.9–3.5 M across runs (noise-dominated) |
| Decoder compacts once per drain, not per command | kills an O(n²) memmove over pipelined batches | `pipeline_set` 0.81 → 0.86 M (+6 %), `pipeline_get` 1.12 → **1.22 M (+9 %)** |
| Zero-allocation command dispatch (`eq_ignore_ascii_case`, no per-command String) | removes an alloc + uppercase pass per op | folded into the pipeline win above |
| Insert ordering: overflow bumped under the home bucket's version BEFORE the spilled entry publishes | makes the miss-skip safe (a reader that can see the spill also sees a positive counter; undercount is impossible — evict/sweep only overcount) | correctness, proven by the new `spilled_keys_never_false_miss` test (52 keys forced to spill, zero false misses) |

**Honest measurement caveat, recorded per the falsifier discipline:** the
hit benches (`get_hit_llc`, `get_hit_dram`, `_ref`, `ttl_read`, even the
unchanged `hash_key`) swung ±50–100 % across three identical-code runs on
this machine (hash_key: 38 M / 68 M / 47 M). Hit-path deltas are therefore
**unresolvable here** — the deciding inputs are the structural wins above.
Making the hit pair trustworthy needs the median-of-N protocol plus the
Phase 1 perf-counter residency rig; until then, treat any single-run hit
number as noise. All 38 tests green after OPT-1.



### Phase 1, final — dual arena + measurement rig (tag `1-final`)

The COMBO arena split landed: probe metadata on its own 64-byte-aligned line
(one line per probe, by construction and by test), entries in a separate
flat array. And the measurement rig exists — an lmbench-style dependent
pointer-chase calibrating this box:

| Working set | ns / dependent load |
| --- | --- |
| 16 KiB (L1) | 1.9 |
| 256 KiB (L2) | 13.9 |
| 8 MiB | 214 (!) |
| 256 MiB (DRAM) | **300** |

Two consequences, recorded honestly:

1. **This is not a server-class memory system.** DRAM at 300 ns/load (spec
   §2.1 assumed ~80–100 ns on server DDR4/5) and 8 MiB already at 214 ns
   means a small, contended last-level cache. Every engine number must be
   read against these floors, not datasheet numbers.
2. **The run-to-run variance is now explained**: background load thrashing a
   small LLC moves hit-path numbers by ±50–100 %. The registry + calibration
   exist precisely so this is visible instead of mysterious.

Grounded results from the same run: `lookup_miss_dram` **4.35 M ops/s**
(230 ns/op ≈ one measured DRAM load — the miss-skip doing its job),
`get_hit_dram_ref` **3.48 M ops/s** zero-copy vs 1.76 M clone (2× on the
DRAM path), `engine/sweep` 639 k ticks/s idle (2.5× — the wheel no longer
walks entry-adjacent lines), `pipeline_set/get` ~0.88 M each.



## The contract — what each phase adds

Completing a phase means, in order:

1. add the phase's new benches (below),
2. re-run the full suite with `--phase <n>` (results append, never overwrite),
3. update the baseline table above,
4. a carried metric regressing > 10 % without a written explanation blocks
   the phase — same discipline as the spec's falsifier column.

| Phase | New benches | Gate they feed |
| --- | --- | --- |
| 1 — SIMD probe, packed 16 B entry | `engine/probe_simd` vs scalar; re-baseline the get_hit LLC/DRAM pair | measured index-hit pair beats this baseline (spec §12: measured, not assumed) |
| 2 — `#[bounded]` | **delivered**: structural gate + compile-time const-nest assertions + coverage test (see phase 2 baseline above) | unbounded hot-path loop fails the build ✓ |
| 3 — resize, timing wheel, epochs | `engine/resize_under_load` (p99 during resize), `engine/sweep` re-baseline, retire-drain cost | no p99.9 spike correlates with maintenance |
| 4 — L0 SPRINT, batching | `engine/l0_sprint_hit` (~5 ns target), `server/batch_pipeline` | MLP ≥ 6.3 validated by perf counters |
| 5 — ORACLE | `oracle/postings_fanout` (invalidations/s), `oracle/invalidate_by_dep` latency | fan-out p99 within MAX_FANOUT |
| 6 — CDC, ladder | `ladder/transition_ms`, token-bucket conformance | feed stall ⇒ transition + heal |
| 7 — DELTA | `delta/apply_us` (20-row set; §12 falsifier: > 20 µs fails), `delta/origin_qps` on repairable sites | origin QPS = 0; audit mismatch = 0 |
| 8 — CHRONOS | `chronos/snapshot_read` overhead vs plain read | no torn read under concurrent writes |
| 9 — prices | `price/decisions_per_s`, `price/regret_vs_fixed` | beats best fixed policy from Phase 0 |
| 10 — SPECTRUM | `spectrum/served_req_per_byte` | improvement on tolerance-declaring workloads |
| 11 — deadlines | `deadline/p999_contract` (overruns without degrade record) | zero budget overruns |
| 12 — flash, mesh | `flash/hit_us`, mesh gossip overhead | 30-day SMART soak within budget |

## Known bench limitations (honesty list)

- Engine get benches include the `Vec<u8>` value clone (real allocation per
  op); a Phase 1 zero-copy read API will re-baseline them downward.
- The LLC/DRAM pair controls working-set size but does not yet verify
  residency with perf counters (LLC-miss/hit) — that instrumentation is part
  of the Phase 1 gate.
- The server suite is one connection on loopback: it measures the TCP path,
  not the engine, and is not comparable to spec §12 in-process targets.
