# Format Spec: Lock Table & SSI State Graph (`samardb-concur`)
# Version: 1

## Overview

SamarDB implements multi-granularity Two-Phase Locking (2PL) alongside Serializable
Snapshot Isolation (SSI based on Cahill et al. 2008).

---

## 1. Lock Modes & Compatibility Matrix

| Requested \ Held | `LOCK_NONE` (0) | `LOCK_SHARED` (1) | `LOCK_EXCLUSIVE` (2) | `LOCK_SIREAD` (3) |
|------------------|-----------------|-------------------|----------------------|-------------------|
| `LOCK_SHARED`    | Grant           | Grant             | Block                | Grant             |
| `LOCK_EXCLUSIVE` | Grant           | Block             | Block                | Grant             |
| `LOCK_SIREAD`    | Grant           | Grant             | Grant                | Grant             |

`LOCK_SIREAD` never blocks and is never blocked by S or X locks; it serves purely as an
instrumentation marker for tracking rw-antidependency edges in the SSI dependency graph.

---

## 2. Lock Table Entry Format

Each resource in the lock table is addressed by its `(page_id, slot_id)` coordinates:

| Field          | Type        | Description                                       |
|----------------|-------------|---------------------------------------------------|
| `page_id`      | `Int`       | Heap page ID (or 0 for table-level)               |
| `slot_id`      | `Int`       | Slot ID on page (or 0 for page-level)             |
| `holder_xids`  | `List[Int]` | Transaction IDs currently holding locks           |
| `holder_modes` | `List[Int]` | Lock mode held by each holder (`1=S`, `2=X`, `3=SIREAD`) |
| `waiter_xids`  | `List[Int]` | Transaction IDs currently waiting                 |
| `waiter_modes` | `List[Int]` | Lock mode requested by each waiter                |

---

## 3. SSI rw-Antidependency Graph Representation

A rw-antidependency (`T1 --rw--> T2`) occurs when `T1` reads a tuple version that `T2`
subsequently modifies or deletes:

| Edge Field   | Type  | Description                                              |
|--------------|-------|----------------------------------------------------------|
| `from_xid`   | `Int` | Transaction `T1` that performed the `SIREAD` read         |
| `to_xid`     | `Int` | Transaction `T2` that wrote the conflicting tuple version |
| `page_id`    | `Int` | Resource page ID where conflict occurred                 |
| `slot_id`    | `Int` | Resource slot ID where conflict occurred                 |

### Dangerous Structure Detection (Pivot Abort Rule)

A transaction `T_pivot` is a pivot in a cycle if:
1. `T_in --rw--> T_pivot` (T_pivot has an incoming rw-antidependency edge)
2. `T_pivot --rw--> T_out` (T_pivot has an outgoing rw-antidependency edge)
3. The overlapping intervals of `T_in`, `T_pivot`, and `T_out` form a potential non-serializable execution.

When a dangerous structure is detected, `T_out` (or `T_pivot`) is flagged with `ERR_SERIALIZATION_FAILURE`
(SQLSTATE `40001`) and aborted.
