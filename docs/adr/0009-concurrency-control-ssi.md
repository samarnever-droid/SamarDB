# ADR 0009 — Concurrency Control & Serializable Snapshot Isolation (SSI)

## Status
Accepted — Phase 8

## Context
SamarDB supports multi-version concurrency control (MVCC) with Snapshot Isolation (Phase 2).
While Snapshot Isolation prevents Dirty Reads, Non-Repeatable Reads, and Phantom Reads, it is
vulnerable to **Write Skew** and **Read-Only Transaction Anomaly**. To provide full ANSI/ISO
`SERIALIZABLE` isolation without the heavy performance degradation of pessimistic Two-Phase
Locking for all reads, we implement Serializable Snapshot Isolation (SSI) based on Cahill et al. (2008).

## Decisions

### 1. Dual-Track Concurrency Architecture
- **Pessimistic 2PL (`LockTable`)**:
  - Used for explicit locking (`SELECT FOR UPDATE`, DDL, schema changes).
  - Supports `LOCK_SHARED` (S) and `LOCK_EXCLUSIVE` (X).
  - Maintains a Wait-For Graph (WFG) to detect and break deadlocks deterministically.
- **Optimistic SSI Engine (`SSIGraph`)**:
  - Used for standard `SERIALIZABLE` transactions.
  - Transactions read without blocking writers (`LOCK_SIREAD` markers).
  - Tracks rw-antidependencies (`from_xid --rw--> to_xid`).
  - Identifies "Dangerous Structures" (`T1 --rw--> T2 --rw--> T3`) and aborts the latest transaction with SQLSTATE `40001 (serialization_failure)`.

### 2. Deadlock Detection in 2PL
- Whenever a lock acquisition blocks:
  - Add an edge `waiter_xid -> holder_xid` to the Wait-For Graph.
  - Run cycle detection (DFS).
  - If a cycle exists, select the youngest transaction as the victim, abort it, and release its held/waiting locks.

### 3. Write-Skew Prevention in SSI
- **Doctor On-Call Example**:
  - Initial state: Dr. A and Dr. B are on-call (count = 2). Rule: at least 1 doctor must remain on-call.
  - `T1` reads on-call count (2), sets Dr. A off-call.
  - Concurrent `T2` reads on-call count (2), sets Dr. B off-call.
  - Under SI: both commit, count = 0 (Violation).
  - Under SSI:
    - `T1` acquires `SIREAD` on Dr. B row.
    - `T2` acquires `SIREAD` on Dr. A row.
    - `T1` writes Dr. A row -> triggers `T2 --rw--> T1`.
    - `T2` writes Dr. B row -> triggers `T1 --rw--> T2`.
    - Cycle `T1 --rw--> T2 --rw--> T1` detected -> `T2` aborted with `40001 serialization_failure`. Invariant preserved!

## Consequences
- Readers never block writers and writers never block readers under standard queries.
- True serializability is guaranteed with zero false negatives for write skew.
