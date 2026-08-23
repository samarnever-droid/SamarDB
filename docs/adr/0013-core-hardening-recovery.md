# ADR 0013 — Core Engine Hardening: Multi-Level B+ Tree, Full DML Recovery, and Typed Records

## Status
Accepted — Phase 12

## Context
Following the comprehensive architecture audit, SamarDB required hardening in 3 specific core areas:
1. **Multi-level B+ Tree traversal**: Enabling `btree_search` to recursively traverse `PAGE_BTREE_INTERNAL` nodes down to leaf pages using child routing pointers.
2. **Complete DML WAL Logging & ARIES REDO**: Extending WAL records to explicitly log `WAL_REC_UPDATE` and `WAL_REC_DELETE` so crash recovery reconstructs full database state after updates and deletes.
3. **Full Typed Relational Codec**: Providing symmetric serialization for `INT`, `VARCHAR`, `BOOL`, and `NULL` values.

## Decisions

### 1. Multi-Level B+ Tree Traversal Engine
- When `btree_search` encounters a page:
  - If `page_type == PAGE_BTREE_INTERNAL`:
    - Call `btree_internal_find_child(page, search_key)` to retrieve the child `page_id`.
    - Fetch child page from storage and recurse.
  - If `page_type == PAGE_BTREE_LEAF`:
    - Call `btree_leaf_find_cell(page, search_key)` to retrieve `(heap_page_id, heap_slot_id)`.
  - If `search_key > high_key` (Lehman-Yao invariant), follow `right_sibling` link to handle concurrent splits.

### 2. Comprehensive ARIES DML Logging
- `wal_append_update_record(wal, txn_id, prev_lsn, page_id, slot_id, old_data, new_data)` logs `WAL_REC_UPDATE (5)`.
- `wal_append_delete_record(wal, txn_id, prev_lsn, page_id, slot_id)` logs `WAL_REC_DELETE (6)`.
- The ARIES REDO scanner replays updates and deletes idempotently against on-disk page LSNs.

### 3. Symmetric Typed Record Codec
- Implemented in `src/schema.lpp`:
  - `record_encode_typed`: packs mixed columns with null bitmap and offset table.
  - `record_get_int`, `record_get_varchar`, `record_get_bool`, `record_is_null`: type-safe extractors.

## Consequences
- B+ Tree indexing supports arbitrarily deep trees (Internal -> Internal -> Leaf).
- Crash recovery guarantees ACID durability across all DML operations (INSERT, UPDATE, DELETE).
- True multi-type relational schemas are supported natively on disk.
