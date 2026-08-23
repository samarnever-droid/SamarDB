# SamarDB Double-Write Buffer (DWB), Savepoints & Deadlock Resolution Specification
# File: docs/format/dwb_and_subxact.md
# Version: 1

## Overview

This specification formalizes three mission-critical database safety systems:
1. **Double-Write Buffer (DWB)**: Torn-page write immunity preventing half-written 4KB disk corruption.
2. **Savepoints & Subtransactions**: Nested transaction control supporting partial rollback without aborting the parent transaction.
3. **Deadlock Victim Selection**: Cost-based cycle breaking in Wait-For graphs to resolve transactional deadlocks.

---

## 1. Double-Write Buffer (DWB) Workflow

```text
Dirty Buffer Pool Frame (8KB)
  ↓
1. Stage in Contiguous Double-Write Buffer (DWB Slots)
  ↓
2. Flush DWB to Sequential Disk Staging Area (fsync)
  ↓
3. In-Place Write to Table Heap Data File
```

### Crash Recovery from Torn Write
If an in-place page write is interrupted by a power loss:
1. Pager reads target page and detects **CRC32C checksum failure**.
2. Recovery engine searches **DWB Slots** for the matching `target_page_id`.
3. Valid 8KB page is copied from DWB to the heap data file and verified.
4. Normal ARIES REDO log replay resumes from `page.lsn`.

---

## 2. Savepoints & Subtransactions

- `SAVEPOINT <name>`: Records current `(txn_id, name, lsn_snapshot, mutation_count)`.
- `ROLLBACK TO SAVEPOINT <name>`: Reverts all heap mutations and uncommitted changes made after the savepoint. Any newer savepoints are discarded.
- `RELEASE SAVEPOINT <name>`: Discards the savepoint descriptor while retaining all changes.

---

## 3. Cost-Based Deadlock Victim Selection

When a directed cycle ($T_A \rightarrow T_B \rightarrow T_A$) is detected in the Wait-For Graph (WFG):
1. The engine computes the cost of each transaction in the cycle (e.g. number of rows modified, locks held, or CPU ticks).
2. The transaction with the **lowest cost** is selected as the victim and aborted, releasing its locks and allowing the remaining transactions to proceed.
