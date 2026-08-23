# ADR 0002: SamarDB Phase 1 Storage Engine & Crash Recovery Architecture

## Status
Accepted / Implemented (2026-08-20)

## Context
SamarDB requires a storage engine matching PostgreSQL on durability (ACID), correctness, and crash recovery, while delivering lower tail latency and deterministic testability in pure L++.

## Decisions

### 1. Slotted Page Layout (`samardb-heap`)
- **Page Size**: Standard 8192 bytes (configurable).
- **Header**: 32-byte fixed header storing CRC32C checksum, 64-bit page LSN, page type, slot count, free space lower/upper pointers, format version, and flags.
- **Item Pointers**: 4-byte line pointers `(offset: u16, length: u16)` growing downwards from byte 32.
- **Tuple Storage**: Stored at page bottom `upper`, growing upwards towards `lower`. Free space is exactly `[lower, upper)`.
- **Compaction**: `heap_page_compact` reclaims fragmented space from deleted slots without modifying external slot IDs.

### 2. Buffer Pool Management (`samardb-pager`)
- **Buffer Pool**: Fixed-frame array with LRU eviction and pin counts.
- **Write-Ahead Logging Invariant (I2)**: Dirty pages cannot be written to disk until all WAL records up to `page.lsn` are flushed to durable storage (`sim_disk_fsync` / `lpp_file_fsync`).
- **Corruption Detection (I8)**: Every page read verifies the CRC32C checksum. Bit rot or torn writes are detected and rejected.

### 3. Write-Ahead Log (`samardb-wal`)
- **Header**: 40-byte record header with CRC32C checksum, monotonic 64-bit LSN, previous LSN, transaction ID, record type, and payload length.
- **Monotonicity (I5)**: LSNs increase strictly monotonically for all logged operations.

### 4. ARIES Crash Recovery (`samardb-recovery`)
- **REDO Idempotence (I6)**: During recovery, the REDO scanner compares `record.lsn` against `page.lsn`. If `record.lsn <= page.lsn`, the record is skipped. If `record.lsn > page.lsn`, the record is applied and `page.lsn` updated. Repeating recovery produces identical state.

## Verification Results
- 40 total automated assertions passing across `test_skeleton.lpp` and `test_phase1.lpp`.
- CRC32C bit-rot rejection verified.
- LRU dirty-frame writeback and persistence verified.
- ARIES REDO idempotence (0 records applied on duplicate scan) verified.
