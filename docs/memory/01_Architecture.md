# 01 Architecture — SamarDB

## System Overview
SamarDB is a relational database engine written in L++. It matches PostgreSQL on durability (ACID), correctness, and crash recovery, while delivering lower tail latency, smaller memory footprint, and deterministic simulation testing.

## Core Invariants
- **I1 (Durability)**: An acknowledged commit is durable against crash, power loss, and kernel panic.
- **I2 (WAL Before Page)**: A dirty page is NEVER written to storage until all WAL records up to `page.lsn` are flushed to durable storage.
- **I3 (Snapshot Isolation)**: Transactions see a consistent snapshot of the database taken at transaction start; readers never block writers, writers never block readers.
- **I5 (LSN Monotonicity)**: Every logged state transition gets a strictly higher LSN.
- **I6 (REDO Idempotence)**: If `record.lsn <= page.lsn`, the record is skipped. If `record.lsn > page.lsn`, it is applied and `page.lsn = record.lsn`.
- **I8 (Self-Verifying Pages & Records)**: Every page and WAL record carries a CRC32C checksum verified on every read.

## Module Boundaries
1. **`samardb-base` (`src/bytes.lpp`)**: Byte buffer allocation, big/little-endian integer codecs, CRC32C checksums, slicing, and memory copying.
2. **`samardb-io` (`src/io.lpp`)**: Deterministic injectable storage interfaces (`SimDisk`) and positional durable file I/O with fault injection.
3. **`samardb-pager` (`src/pager.lpp`)**: 8KB page frame management, 32-byte PostgreSQL-style header, LRU buffer pool, pin counting, and dirty writeback.
4. **`samardb-wal` (`src/wal.lpp`)**: Write-ahead log record encoding/decoding, monotonic LSN sequencing, and log flushing.
5. **`samardb-heap` (`src/heap.lpp`)**: Slotted page tuple storage, item pointer arrays, tuple insertion/deletion, and defragmentation compaction.
6. **`samardb-recovery` (`src/recovery.lpp`)**: ARIES crash recovery engine with deterministic REDO scanning and idempotence guarantees.
7. **`samardb-tx` (`src/tx.lpp`)**: Transaction manager, monotonic Transaction ID allocation, bitwise 2-bit CLOG status table (`IN_PROGRESS`, `COMMITTED`, `ABORTED`), and point-in-time Snapshot generation.
8. **`samardb-mvcc` (`src/mvcc.lpp`)**: 24-byte MVCC tuple headers (`xmin`, `xmax`, `cid`, `infomask`, `payload_len`), tuple codecs, and PostgreSQL-style visibility evaluation algorithm.
9. **`samardb-vacuum` (`src/vacuum.lpp`)**: Non-blocking MVCC update/delete, snapshot-aware TID scan, and VACUUM engine for reclaiming dead tuple versions older than the oldest active transaction.
10. **`samardb-btree` (`src/btree.lpp`)**: Lehman-Yao B-link tree on-disk format, binary search, 50/50 page splits, right-sibling links, point lookups, and range scans.
11. **`samardb-schema` (`src/schema.lpp`)**: Relational table schema descriptors, column data types (`INT`, `VARCHAR`, `BOOL`), binary row record codecs, and null bitmaps.
12. **`samardb-parser` (`src/parser.lpp`)**: SQL lexer / tokenizer, recursive descent parser, and AST statements (`CREATE TABLE`, `INSERT`, `SELECT`, `UPDATE`, `DELETE`, `JOIN`, `WHERE`).
13. **`samardb-exec` (`src/exec.lpp`)**: System catalog and Volcano execution pipeline (`SeqScan`, `IndexScan`, `Filter`, `Project`, `NestedLoopJoin`, `DML`).
14. **`samardb-opt` (`src/opt.lpp`)**: Cost-based query optimizer — `StatCatalog` (table/column/index statistics), integer fixed-point selectivity estimation, Selinger-style DP join ordering, `SeqScan` vs `IndexScan` physical scan selection, `PlanNode` cost-annotated physical plan tree, and `explain_plan` for human-readable plan output.
15. **`samardb-pgwire` (`src/pgwire.lpp`)**: PostgreSQL wire protocol v3.0 frontend — `SimSocket` injectable socket (in_buf/out_buf reference handles), big-endian frame encoder/decoder (`pgwire_write_byte/int16be/int32be/str`), all 7 message builders (`AuthOK`, `BackendKeyData`, `ReadyForQuery`, `RowDescription`, `DataRow`, `CommandComplete`, `ErrorResponse`), startup handshake state machine (`pgwire_do_startup`), and query result streamer.
16. **`samardb-raft` (`src/raft.lpp`)**: Distributed consensus & replication engine — `SimNetwork` deterministic message queue, `RaftNode` state machine (Follower, Candidate, Leader), `RequestVote` / `VoteReply` election protocol with quorum counting, `AppendEntries` / `AppendReply` log replication and quorum-commit, term monotonicity, and leader crash / re-election recovery.
17. **`samardb-concur` (`src/concur.lpp`)**: Concurrency control & Serializable Snapshot Isolation (SSI) — `LockTable` with 2PL Shared/Exclusive lock modes, lock acquisition/release and waiter promotion, Wait-For Graph deadlock cycle detection, and Cahill SSI engine with `SIREAD` locks and rw-antidependency tracking for Write Skew anomaly prevention.
18. **`samardb-bench` (`src/bench.lpp`)**: Microbenchmark & performance measurement harness — `BenchStats` latency histogram accumulator, exact integer list sorting, percentile calculations (`p50`, `p95`, `p99`), and microbenchmark workloads for B+ Tree lookups, Slotted Page heap inserts, Volcano query pipelines, and Selinger DP query optimization.
19. **`samardb-chaos` (`src/chaos.lpp`)**: Chaos testing & fault injection engine — `chaos_inject_torn_write`, `chaos_inject_bitrot`, ARIES self-healing recovery verifier, and split-brain network partition quorum proofs.
20. **`samardb-agg` (`src/agg.lpp`)**: Analytical query & aggregation engine — `GroupTable` hash/bucket accumulator, global reductions (`COUNT`, `SUM`, `AVG`, `MIN`, `MAX`), `GROUP BY` grouping, `HAVING` aggregate predicate filtering, `ORDER BY` (ASC/DESC), `LIMIT`/`OFFSET` windowing, and `DISTINCT` row deduplication.
21. **`samardb-hardened` (`src/btree.lpp`, `src/wal.lpp`, `src/recovery.lpp`, `src/schema.lpp`)**: Multi-level B+ Tree internal node child routing, comprehensive ARIES DML logging (`WAL_REC_UPDATE` & `WAL_REC_DELETE`), and full variable-length typed relational codecs (`INT`, `VARCHAR`, `BOOL`, `NULL`).
22. **`samardb-catalog` (`src/catalog.lpp`)**: Multi-table system catalog & dynamic DDL engine — `SystemCatalog`, `catalog_create_table`, `catalog_drop_table` (with column & index cascading), `catalog_create_index`, `catalog_drop_index`, `catalog_alter_add_column` (schema evolution), and `catalog_to_table_schema` runtime bridge.
23. **`samardb-constraints` (`src/constraints.lpp`)**: Relational integrity, foreign keys & unique constraint engine — `ConstraintManager`, `ForeignKey`, `CheckConstraint`, `UniqueConstraint`, pre-WAL insertion validation (NOT NULL, CHECK, UNIQUE, FK), and referential delete action state machine (`RESTRICT`, `CASCADE`, `SET_NULL`).
24. **`samardb-safety` (`src/safety.lpp`)**: Double-Write Buffer (DWB / torn-write immunity), nested subtransaction savepoints (`SAVEPOINT`, `ROLLBACK TO SAVEPOINT`, `RELEASE SAVEPOINT`), and cost-based deadlock victim selection.
25. **`samardb-adversarial-chaos` (`src/test_father_chaos.lpp`, `src/test_stress_load.lpp`)**: Dark-corner boundary testing (0-byte/oversized tuples, page saturation, negative B+ tree keys, double-fault recovery, write-skew SSI detection, memory slicing) and production stress load.
