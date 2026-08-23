# ADR 0016: Double-Write Buffer, Savepoints & Deadlock Victim Selection
# Status: ACCEPTED
# Date: 2026-08-20

## Context
1. **Torn Writes**: In standard OS filesystems, 8KB database page writes are non-atomic. A sudden crash during a write leaves 4KB of old data and 4KB of new data, corrupting the page permanently.
2. **Partial Failures**: Applications need granular error handling where a single failing SQL statement can be rolled back without losing the entire transaction's prior work.
3. **Deadlocks**: Mutually waiting transactions must be resolved automatically by aborting the least costly transaction.

## Decision
1. **Double-Write Buffer (DWB)**: Implement an in-memory and on-disk staging buffer (`DwbManager`) that sequesters dirty pages before writing them to the main heap file.
2. **Savepoint Stack**: Maintain a per-transaction savepoint list (`SavepointManager`) recording `(lsn, mutation_count)` boundaries for partial undo.
3. **Cost-Based Deadlock Victim Selection**: Invert the traditional random victim abort by selecting the transaction with the lowest active work footprint.

## Consequences
- **Positive**: 100% immunity to torn-write data loss on all storage media.
- **Positive**: Full SQL standard compliance for nested subtransactions and savepoint rollback.
- **Positive**: Optimal throughput during concurrent deadlock resolution.
