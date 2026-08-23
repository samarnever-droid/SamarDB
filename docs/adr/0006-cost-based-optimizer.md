# ADR 0006 — Cost-Based Query Optimizer (`samardb-opt`)

## Status
Accepted — Phase 5

## Context

Phase 4 delivered a Volcano execution pipeline where every SELECT produces the same
physical plan regardless of the data distribution. A query `SELECT * FROM a JOIN b JOIN c`
always performs the join in left-to-right parse order, and every column predicate always
chooses SeqScan. This is correct but suboptimal for any non-trivial data set.

A cost-based optimizer selects the physical plan with the lowest estimated execution cost
before the query hits the executor, based on catalog statistics gathered from the live data.

## Decision

### 1. Statistics Catalog

Two complementary statistics structures are maintained per ANALYZE cycle (or on INSERT in
test scenarios):

- **`TableStats`**: Per-table tuple count, dead tuple count, and heap page count.
- **`ColumnStats`**: Per-column NDV (number of distinct values), null fraction, and a
  multi-bucket frequency histogram for range predicate selectivity estimation.

Statistics are kept in in-memory parallel lists keyed by `(table_name, col_name)`.

### 2. Selectivity Estimation

Selectivity `sel(pred)` is a dimensionless fraction in `[1/n_tuples, 1.0]`:

- Equality `col = v`: `1 / n_distinct`.
- Inequality `col != v`: `(n_distinct - 1) / n_distinct`.
- Range `col < v`: fraction of histogram buckets strictly below `v`.
- Complementary ranges `col >= v`: `1 - sel(col < v) + 1/n_distinct`.

For multi-predicate conjunctions the selectivities are multiplied (independence
assumption — the same as PostgreSQL's default model).

### 3. Operator Cost Model

Every plan node carries an estimated cost (dimensionless units, tunable constants):

| Constant        | Default | Meaning                                             |
|-----------------|---------|-----------------------------------------------------|
| PAGE_FETCH_COST | 1.0     | Cost per 8 KB page fetched sequentially             |
| IDX_PAGE_COST   | 1.0     | Cost per B+ Tree page traversed                     |
| TUPLE_FETCH_COST| 0.5     | Cost per random tuple fetch from heap               |
| CPU_COST        | 0.01    | Cost per tuple processed by CPU (filter, project)   |

- **SeqScan**: `n_pages × PAGE_FETCH_COST`
- **IndexScan**: `n_levels × IDX_PAGE_COST + sel × n_tuples × TUPLE_FETCH_COST`
- **Filter**: `input_cost + sel × n_tuples × CPU_COST`
- **Project**: `input_cost + n_output_rows × CPU_COST`
- **NestedLoopJoin**: `outer_cost + n_outer_rows × inner_cost`

### 4. Scan Selection (SeqScan vs. IndexScan)

For any filtered scan over table T on column C with predicate P:

1. Compute `sel = selectivity(P, ColumnStats[T.C])`.
2. Compute `cost_seq = TableStats[T].n_pages × PAGE_FETCH_COST`.
3. If an index exists on T.C, compute `cost_idx = n_levels × IDX_PAGE_COST + sel × n_tuples × TUPLE_FETCH_COST`.
4. Choose `IndexScan` if `cost_idx < cost_seq`, else `SeqScan`.

### 5. Join Ordering (Selinger DP)

For queries with N join relations, the optimizer enumerates left-deep join trees using
Selinger's bottom-up dynamic programming algorithm:

```
dp[{R}]      = best_scan_plan(R)    for each base relation R
dp[S ∪ {R}]  = min over R in S of:
                  plan = NLJ(dp[S \ {R}], best_scan_plan(R))
                  cost = cost(dp[S \ {R}]) + n_outer × cost(best_scan_plan(R))
```

For Phase 5, N ≤ 4 relations are supported. For N > 4, a greedy "smallest first" heuristic
is applied instead of full DP enumeration.

### 6. Predicate Pushdown

Before join ordering, filter predicates are pushed as far down the operator tree as
possible:

1. Identify every `WHERE col op val` predicate in the query AST.
2. Determine which base table each predicate references.
3. Push each predicate into the scan plan for its base table (wrapped as a Filter node),
   reducing the cardinality estimate before join nodes.

## Consequences

- Queries with selective predicates and matching B+ Tree indexes will automatically prefer
  `IndexScan` over `SeqScan`.
- Multi-table joins will be reordered so the smallest intermediate result sets are produced
  first, minimizing NestedLoopJoin fan-out.
- Predicate pushdown ensures filters reduce row counts as early as possible in the plan.
- Statistics accuracy degrades between ANALYZE runs; a future ANALYZE trigger on high write
  rates is deferred to Phase 7.
