# Format Spec: Performance Benchmark Metrics (`samardb-bench`)
# Version: 1

## Overview

SamarDB includes an integrated benchmark and performance harness to track latency,
throughput, buffer pool efficiency, and execution time across all storage and query subsystems.

---

## 1. Benchmark Result Record Format

Each benchmark run produces a summary record containing the following fields:

| Field            | Type   | Description                                            |
|------------------|--------|--------------------------------------------------------|
| `bench_name`     | `Str`  | Workload name (e.g., `PointLookup_10k`, `RangeScan`)   |
| `op_count`       | `Int`  | Total operations executed                              |
| `total_ticks`    | `Int`  | Total simulated CPU clock / time units elapsed         |
| `ops_per_sec`    | `Int`  | Calculated throughput (operations scaled to 1s)        |
| `p50_latency`    | `Int`  | 50th percentile (median) latency                       |
| `p95_latency`    | `Int`  | 95th percentile tail latency                           |
| `p99_latency`    | `Int`  | 99th percentile tail latency                           |
| `cache_hits`     | `Int`  | Buffer pool / cache hits                               |
| `cache_misses`   | `Int`  | Buffer pool / cache misses                             |
| `hit_ratio_pct`  | `Int`  | Cache hit ratio percentage (0..100)                    |

---

## 2. Supported Benchmark Workloads

1. **`BENCH_POINT_LOOKUP` (1)**: Single-key B+ Tree index lookup against 1,000 indexed records.
2. **`BENCH_RANGE_SCAN` (2)**: Lehman-Yao B+ Tree ordered range scan traversing sibling links across pages.
3. **`BENCH_HEAP_INSERT` (3)**: Slotted-page heap tuple insertion with defragmentation and free space accounting.
4. **`BENCH_VOLCANO_QUERY` (4)**: End-to-end Volcano relational execution (`SeqScan` -> `Filter` -> `Project`).
5. **`BENCH_OPTIMIZER_PLAN` (5)**: Selinger DP join enumeration and physical plan cost estimation.
