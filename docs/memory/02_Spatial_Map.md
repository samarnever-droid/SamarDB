# 02 Spatial Map — SamarDB

## Component Locations

### `src/bytes.lpp`
- Structure: Functions + Builtin wrappers
- Anchor: `Bytes`, `bytes_alloc`, `bytes_get64le`, `bytes_set64le`, `bytes_crc32`
- Connects to: `01_Architecture.md::samardb-base`

### `src/io.lpp`
- Structure: `SimDisk`, `DiskFaultConfig`, `sim_disk_pread`, `sim_disk_pwrite`, `sim_disk_fsync`
- Anchor: Deterministic injectable I/O layer with bit-rot and fsync fault injection
- Connects to: `01_Architecture.md::samardb-io`

### `src/pager.lpp`
- Structure: `PageHeader`, `BufferFrame`, `BufferPool`
- Anchor: `page_header_encode`, `page_header_decode`, `buffer_pool_fetch_page`, `buffer_pool_unpin_page`, `buffer_pool_flush_all`
- Connects to: `01_Architecture.md::samardb-pager`

### `src/wal.lpp`
- Structure: `WalRecordHeader`, `WalRecord`, `WalManager`
- Anchor: `wal_record_encode`, `wal_record_decode`, `wal_append_record`, `wal_flush`
- Connects to: `01_Architecture.md::samardb-wal`

### `src/heap.lpp`
- Structure: `ItemPointer`, `heap_page_init`, `heap_page_insert`, `heap_page_read`, `heap_page_delete`, `heap_page_compact`
- Anchor: Slotted page tuple layout and defragmentation
- Connects to: `01_Architecture.md::samardb-heap`

### `src/recovery.lpp`
- Structure: `RecoveryStats`, `recovery_redo_scan`
- Anchor: ARIES REDO pass with LSN idempotence checking
- Connects to: `01_Architecture.md::samardb-recovery`
