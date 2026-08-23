# Format Spec: Analytical Aggregations & Grouping (`samardb-agg`)
# Version: 1

## Overview

SamarDB Phase 11 specifies the accumulator state format and in-memory representation for
SQL analytical aggregate functions (`COUNT`, `SUM`, `AVG`, `MIN`, `MAX`), `GROUP BY` grouping
buckets, `HAVING` filters, `ORDER BY` sorting, and `LIMIT`/`OFFSET` windowing.

---

## 1. Aggregate Function Codes

| Code | Aggregate Name | Description                                                  |
|------|----------------|--------------------------------------------------------------|
| `1`  | `AGG_COUNT`    | Number of matching rows in group (`count`)                   |
| `2`  | `AGG_SUM`      | Arithmetic sum of column values (`sum`)                      |
| `3`  | `AGG_AVG`      | Average value (`sum / count`), computed as integer or ratio |
| `4`  | `AGG_MIN`      | Minimum value observed in group (`min_val`)                  |
| `5`  | `AGG_MAX`      | Maximum value observed in group (`max_val`)                  |

---

## 2. Group Accumulator Record

Each unique group key in a `GROUP BY` query maintains a 48-byte accumulator state:

| Field       | Type   | Offset | Length | Description                                   |
|-------------|--------|--------|--------|-----------------------------------------------|
| `group_key` | `Int`  | `0`    | `8`    | Grouping column value (or 0 for global agg)   |
| `count`     | `Int`  | `8`    | `8`    | Number of tuples accumulated                  |
| `sum`       | `Int`  | `16`   | `8`    | Sum of target column values                   |
| `min_val`   | `Int`  | `24`   | `8`    | Minimum value seen                            |
| `max_val`   | `Int`  | `32`   | `8`    | Maximum value seen                            |
| `is_init`   | `Bool` | `40`   | `8`    | Whether accumulator has observed >= 1 row     |

---

## 3. Sort Order Descriptors

| Code | Order Code   | Description                                            |
|------|--------------|--------------------------------------------------------|
| `0`  | `ORDER_NONE` | Unsorted / insertion order                             |
| `1`  | `ORDER_ASC`  | Ascending order (lowest to highest)                    |
| `2`  | `ORDER_DESC` | Descending order (highest to lowest)                   |
