# 11 — Targets, with basis and falsifier

The quantitative contract. `✗` marks AEGIS rows that do not survive arithmetic.
Two of them — `TTL = ∞` and `0 knobs` — are not optimistic numbers; they are
removals of the mechanisms that make the system safe when its model is wrong.

| Metric | v4 HELIOS | v7 AEGIS | v5 MERIDIAN | Basis | Falsifier |
|---|---|---|---|---|---|
| L0 hot GET | ~5 ns | ~5–6 ns | ~5–6 ns | derived | L1-resident load-to-use > 2 ns on target part |
| Index hit, DRAM-resident | 150–200 ns (2 misses) | 12–35 ns ✗ | 80–110 ns | derived §2.1 | dependent random DRAM load < 60 ns |
| Index hit, LLC-resident | — | 12–35 ns | 18–35 ns | derived §2.1 | LLC load-to-use > 20 ns at 3 GHz |
| Atomics on read path | 0 | 0 | 0 | derived | any RMW in emitted probe path |
| Cache lines per index hit | 2→1 (dual arena) | 1 | 1 | derived | perf shows > 1.1 LLC-miss/hit |
| Batched throughput | 25 M/core | — | 25 M/core | derived, MLP-bound | per-core random bandwidth < 8 GB/s |
| Rehash pause | none | none | none | bound | p99.9 spike correlated with resize |
| Maintenance stall | per-mechanism | work-credit | compiler-proved #[bounded] | bound | hot-path loop without annotation compiles |
| Request p99.9 | not bounded | not bounded | contractual — degrade before exceed | bound §9 | completed request exceeding budget without degrade record |
| Restart to warm | ~0 ms (shm) | ~0 ms | ~0 ms / <900 ms reboot | derived | shm image rejected on restart |
| Flash lifetime | token bucket | governed | priced, invalidation-aware | derived §3.2–3.3 | SMART burn-down exceeding budget over 30-day soak |
| Hit-ratio reference | vs Belady | within 1.5 % of Belady | served-req/byte under tolerance | §7.3 | exact-only MERIDIAN loses to Caffeine |
| Origin QPS, repairable | bounded by bucket | coalesced, bounded | zero | derived §5.1 | any origin query for a maintained, resident entry |
| Origin CPU, 10 k hot entries | ~20 cores | ~20 cores | 0 origin / 0.04 local | derived §5.1 | delta-apply > 20 µs on a 20-row set |
| Cross-entry consistency | none (disclosed) | none | snapshot-isolated read sets | §6 | two entries in one snapshot reflect different commit prefixes |
| p99 staleness | ≤ cdc_lag, ladder-clamped | ~35 ms ✗ | ≤ max(measured lag, clamp), exported | bound §2.2 | staleness exceeding clamp without ladder transition |
| Operator knobs | ~40 | 0 ✗ | 1 SLO + 6 structural | §2.3 | a seventh structural knob no SLO can derive |
| Self-healing after provenance bug | TTL backstop | TTL = ∞ ✗ | TTL backstop, ∞ rejected at compile time | §2.7 | missed invalidation persisting beyond class TTL |

## The falsifier duty

The acceptance gate for each phase is that someone actively tried to trigger the
falsifier and failed. A benchmark suite proves the system works on the cases you
thought of; the falsifier column is the list of cases that would prove it doesn't
— and running those is the only part of measurement that can change your mind.
