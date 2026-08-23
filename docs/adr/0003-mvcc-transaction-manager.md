# ADR 0003: Multi-Version Concurrency Control (MVCC) & Snapshot Isolation

## Status
Accepted / In-Progress (2026-08-20)

## Context
Relational database workloads require concurrent transactions without readers blocking writers and writers blocking readers. SamarDB adopts PostgreSQL-style Multi-Version Concurrency Control (MVCC) with Snapshot Isolation.

## Decisions

### 1. Snapshot Isolation Model
- **Snapshot Representation**: A snapshot is defined by `Snapshot(xmin: Int, xmax: Int, active_txns: List[Int])`:
  - `xmin`: Earliest transaction that was still active when the snapshot was taken. All `tx < xmin` are committed and visible (if not deleted).
  - `xmax`: First unassigned transaction ID at snapshot creation time. All `tx >= xmax` are invisible to this snapshot.
  - `active_txns`: List of in-flight transactions at snapshot time between `[xmin, xmax)`. Any `tx` in `active_txns` is invisible.

### 2. Visibility Evaluation Algorithm
Given a tuple with header `(xmin, xmax, cid, infomask)`:
1. **Creation Visibility (`xmin`)**:
   - If `xmin == current_txn_id`: visible if created before current `cid` and not aborted.
   - If `xmin` is in `active_txns` or `xmin >= snapshot.xmax` or `CLOG(xmin) == ABORTED`: **NOT VISIBLE**.
   - If `CLOG(xmin) == COMMITTED` and `xmin < snapshot.xmax`: creation is visible -> check `xmax`.
2. **Deletion Visibility (`xmax`)**:
   - If `xmax == 0` or `CLOG(xmax) == ABORTED`: **VISIBLE**.
   - If `xmax == current_txn_id`: **NOT VISIBLE** (deleted by self).
   - If `xmax` is in `active_txns` or `xmax >= snapshot.xmax` or `CLOG(xmax) == IN_PROGRESS`: **VISIBLE** (delete not yet committed in snapshot).
   - If `CLOG(xmax) == COMMITTED` and `xmax < snapshot.xmax` and `xmax` not in `active_txns`: **NOT VISIBLE** (deleted before snapshot).

### 3. Space Reclamation via VACUUM
- Tuples whose `xmax` is committed and `xmax < oldest_active_xmin` are unconditionally invisible to all existing and future transactions.
- `heap_vacuum_page` marks these dead tuple slots and defragments page storage using `heap_page_compact`.
