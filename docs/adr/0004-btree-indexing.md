# ADR 0004: B+ Tree Indexing Engine Architecture

## Context
SamarDB requires primary and secondary index lookup capabilities to support \(O(\log N)\) point queries and \(O(\log N + K)\) ordered range scans. Relational query engines rely on B+ Trees to satisfy `WHERE id = ?`, `WHERE age BETWEEN x AND y`, and index-supported joins.

## Decisions

1. **Lehman-Yao B-link Tree Variant**:
   - Each node contains a `right_sibling` pointer (stored at bytes `24..28` in the page header).
   - Right sibling links eliminate deadlock during concurrent node splits and allow readers to navigate rightward without re-traversing from the root.

2. **Unified 8KB Slotted Node Architecture**:
   - Both Leaf and Internal nodes use the standard SamarDB 8KB page format with 32-byte header and CRC32C checksums.
   - Cells are stored from the bottom up (`upper`), and 2-byte cell pointers are stored from top down (`lower`).
   - Maintaining sorted order in the cell pointer array allows fast binary search over in-memory nodes while keeping memory compaction simple.

3. **Point Lookup & Range Scan Iterators**:
   - `btree_search(pool, root_page_id, key)`: Traverses internal nodes down to the target leaf and performs binary search.
   - `btree_range_scan(pool, root_page_id, min_key, max_key)`: Finds the start leaf, iterates across cells, and crosses `right_sibling` page links seamlessly until `max_key` is reached.

4. **Split Protocol & Root Growth**:
   - When a leaf or internal node reaches capacity, it performs a 50/50 split into a new page.
   - When the root node splits, a new root page is allocated with `PAGE_BTREE_INTERNAL`, pointing to the two halves.

## Consequences
- **Positive**:
  - Predictable \(O(\log N)\) depth (fanout \(\approx 450\), 3 levels can index over 90 million tuples).
  - Exact match and ordered range scans integrate cleanly with the query engine (Phase 4).
  - Zero lock contention on right-walking readers.
- **Accepted Tradeoff**:
  - Node deletion uses tombstone/compaction rather than immediate underflow rebalancing (standard practice in high-performance engines like PostgreSQL and SQLite).
