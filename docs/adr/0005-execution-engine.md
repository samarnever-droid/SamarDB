# ADR 0005: Execution Engine & SQL Parser Architecture

## Context
SamarDB requires a relational query execution engine capable of evaluating SQL DDL (`CREATE TABLE`), DML (`INSERT`, `UPDATE`, `DELETE`), and queries (`SELECT ... FROM ... WHERE ... JOIN ...`). The execution engine must integrate directly with the storage engine (Phase 1), transaction manager/MVCC (Phase 2), and B+ Tree indexes (Phase 3).

## Decisions

1. **Volcano Iterator Execution Model**:
   - Query operators implement pull-based pipelining (`open`, `next`, `close`).
   - `next()` yields one tuple at a time through the operator tree, achieving minimal memory footprint and streaming pipeline execution.

2. **Operator Hierarchy**:
   - `SeqScan`: Scans all heap pages allocated to a table, filtering tuples through the active MVCC Snapshot.
   - `IndexScan`: Uses B+ Tree point lookups or range scans to directly retrieve matching `(heap_page_id, heap_slot_id)` Row IDs, reading only the necessary heap pages.
   - `Filter`: Evaluates predicates (`=`, `!=`, `<`, `<=`, `>`, `>=`) on input tuple streams.
   - `Project`: Extracts selected column indices/expressions into output records.
   - `NestedLoopJoin`: Computes inner joins across two relation pipelines with predicate matching.
   - `InsertExec` / `UpdateExec` / `DeleteExec`: Coordinates MVCC writes, WAL logging, and B+ Tree index maintenance atomically.

3. **Recursive Descent SQL Parser**:
   - Handcrafted lexer tokenizing SQL keywords (`SELECT`, `INSERT`, `UPDATE`, `DELETE`, `CREATE`, `TABLE`, `FROM`, `WHERE`, `JOIN`, `ON`, `AND`, `OR`), identifiers, integers, strings, and operators.
   - Deterministic AST representation avoiding dynamic reflection or heavy external dependencies.

4. **System Catalog**:
   - In-memory and persistent catalog mapping table names to column definitions, heap root page IDs, and secondary B+ Tree index root page IDs.

## Consequences
- **Positive**:
  - Full relational pipeline from SQL text to disk I/O.
  - Zero-copy tuple passing where possible; minimal memory overhead.
  - Snapshot Isolation is preserved end-to-end through operator pipelines.
