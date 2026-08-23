# Format Spec: Catalog Statistics (`samardb-opt`)
# Version: 1

## Overview

The cost-based optimizer requires persistent table and column statistics. Statistics
are stored in the System Catalog memory structures (not yet flushed to disk in Phase 5).
This document specifies the in-memory and eventual on-disk binary layout for table and
column statistics used by the Selinger-style cost model.

---

## 1. Table Statistics (`TableStats`)

Tracks per-table cardinality information needed for join ordering and scan cost.

| Field        | Type  | Description                                  |
|--------------|-------|----------------------------------------------|
| `table_name` | Str   | Logical table name (matches Catalog schema)  |
| `n_pages`    | Int64 | Number of heap pages (extents of 8 KB)       |
| `n_tuples`   | Int64 | Estimated live tuple count (after VACUUM)    |
| `n_dead`     | Int64 | Estimated dead (MVCC-invisible) tuple count  |

Cost formula for a full SeqScan:

```
cost_seqscan(T) = n_pages(T) * PAGE_FETCH_COST
```

Default `PAGE_FETCH_COST = 1.0` (dimensionless; scale factor for future disk vs. RAM tiers).

---

## 2. Column Statistics (`ColumnStats`)

Tracks per-column NDV (number of distinct values), null fraction, and a step histogram
for selectivity estimation of equality and range predicates.

| Field          | Type      | Description                                      |
|----------------|-----------|--------------------------------------------------|
| `table_name`   | Str       | Parent table name                                |
| `col_name`     | Str       | Column name                                      |
| `n_distinct`   | Int64     | Estimated distinct value count (NDV)             |
| `null_frac`    | Int64     | Null fraction × 10000 (fixed-point percent)      |
| `hist_lo`      | Int64[]   | Lower bounds of each histogram bucket            |
| `hist_hi`      | Int64[]   | Upper bounds of each histogram bucket            |
| `hist_freq`    | Int64[]   | Estimated row count in each histogram bucket     |

### Selectivity Formulas

| Predicate    | Selectivity                                 |
|--------------|---------------------------------------------|
| `col = v`    | `1 / n_distinct`                            |
| `col != v`   | `(n_distinct - 1) / n_distinct`             |
| `col < v`    | bucket fraction below `v` in histogram      |
| `col <= v`   | bucket fraction at/below `v` in histogram   |
| `col > v`    | `1 - (col < v selectivity)`                 |
| `col >= v`   | `1 - (col <= v selectivity) + (1/n_distinct)` |

All selectivities are clamped to `[1/n_tuples, 1.0]`.

---

## 3. IndexStats

| Field        | Type  | Description                              |
|--------------|-------|------------------------------------------|
| `table_name` | Str   | Table name                               |
| `col_name`   | Str   | Indexed column name                      |
| `n_levels`   | Int64 | B+ Tree height (root to leaf depth)      |
| `n_pages`    | Int64 | Total B+ Tree page count                 |

Cost formula for an IndexScan point lookup:

```
cost_indexscan(idx, sel) = n_levels(idx) * IDX_PAGE_COST + sel * n_tuples(T) * TUPLE_FETCH_COST
```

Default constants: `IDX_PAGE_COST = 1.0`, `TUPLE_FETCH_COST = 0.5`.

---

## 4. Plan Node Costs

The optimizer annotates each plan node with an estimated cost:

| Node Type      | Cost Formula                                                            |
|----------------|-------------------------------------------------------------------------|
| SeqScan        | `n_pages(T) * PAGE_FETCH_COST`                                          |
| IndexScan      | `n_levels(I) * IDX_PAGE_COST + sel * n_tuples(T) * TUPLE_FETCH_COST`   |
| Filter         | `input_cost + sel * n_tuples(T) * CPU_COST`                             |
| Project        | `input_cost + n_output_rows * CPU_COST`                                 |
| NestedLoopJoin | `outer_cost + n_outer * inner_cost`                                     |

Default: `CPU_COST = 0.01`.

Join order is determined by enumerating left-deep plan trees and selecting minimum total estimated cost via Selinger dynamic programming.
