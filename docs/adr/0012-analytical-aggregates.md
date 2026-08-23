# ADR 0012 — Analytical Aggregations, GROUP BY, HAVING, and Sort/Windowing

## Status
Accepted — Phase 11

## Context
Standard OLTP query engines process individual rows. Analytical SQL queries require group
accumulation (`GROUP BY`), aggregate reduction (`COUNT`, `SUM`, `AVG`, `MIN`, `MAX`),
group-level filtering (`HAVING`), sorting (`ORDER BY`), and windowed pagination (`LIMIT`/`OFFSET`).

## Decisions

### 1. Hash & Group Accumulator (`GroupTable`)
- Tuples are streamed from Volcano operators (`SeqScan`, `Filter`, `Join`).
- For each tuple, the group key is evaluated:
  - If a group bucket exists for `group_key`, update `count`, `sum`, `min`, `max`.
  - Otherwise, create a new group bucket initialized with the row's values.
- For global aggregates (no `GROUP BY` clause), all rows map to `group_key = 0`.

### 2. HAVING Filter Evaluation
- Post-aggregation filtering evaluates predicates against the calculated aggregate value (e.g. `HAVING SUM(salary) > 50000` or `HAVING COUNT(*) >= 5`).

### 3. ORDER BY & LIMIT / OFFSET Pipeline
- **`ORDER BY`**: In-memory comparator-based sorting of tuple rows or aggregated group rows by specified column in `ASC` or `DESC` order.
- **`LIMIT n OFFSET m`**: Direct slice projection over the sorted output stream, discarding the first `m` rows and taking up to `n` rows.

## Consequences
- Full support for SQL-92 analytical queries without needing external processing engines.
- Zero copy overhead for accumulator updates.
