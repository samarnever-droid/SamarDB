# Write-Ahead Log (WAL) Format Specification (v1)

Format version: `1`  
Default segment size: `16777216` bytes (16 MiB preallocated and zero-filled)

---

## 1. WAL Record Binary Layout

Every WAL record written to disk adheres to the following fixed-header format:

| Offset | Size (bytes) | Type | Field | Description |
|---|---|---|---|---|
| `0` | `4` | `u32` | `checksum` | CRC32C computed over bytes `[4, total_len)` |
| `4` | `4` | `u32` | `total_len` | Total record length including header and payload |
| `8` | `8` | `u64` | `lsn` | Monotonically increasing Log Sequence Number (byte offset) |
| `16` | `8` | `u64` | `txn_id` | 64-bit transaction identifier |
| `24` | `8` | `u64` | `prev_lsn` | LSN of previous record written by this transaction (`0` if first) |
| `32` | `1` | `u8` | `rec_type` | Record type discriminator (see WAL Record Types) |
| `33` | `1` | `u8` | `flags` | Bitmask flags (e.g. 0x01=compressed payload) |
| `34` | `2` | `u16` | `format_version`| Layout version (must be `1`) |
| `36` | `4` | `u32` | `payload_len` | Length of payload immediately following header |
| `40` | `payload_len` | `bytes` | `payload` | Operation-specific binary data |

---

## 2. WAL Record Types

| ID | Name | Payload Contents |
|---|---|---|
| `1` | `WAL_TXN_BEGIN` | Timestamp, isolation level |
| `2` | `WAL_TXN_COMMIT` | Commit timestamp, commit LSN |
| `3` | `WAL_TXN_ABORT` | Abort reason |
| `4` | `WAL_HEAP_INSERT` | Relation ID, Page ID, Slot ID, Tuple Bytes |
| `5` | `WAL_HEAP_UPDATE` | Relation ID, Old Page/Slot, New Page/Slot, Tuple Delta |
| `6` | `WAL_HEAP_DELETE` | Relation ID, Page ID, Slot ID, xmax |
| `7` | `WAL_BTREE_INSERT` | Index ID, Page ID, Key Bytes, Heap TID |
| `8` | `WAL_BTREE_SPLIT` | Index ID, Left Page ID, Right Page ID, Split Key |
| `9` | `WAL_FPI` | Full-page image (Page ID, raw page bytes) for torn-write recovery |
| `10` | `WAL_CHECKPOINT` | Last checkpoint LSN, active txns table, dirty pages table |
| `11` | `WAL_DDL` | Transactional catalog mutation command payload |

---

## 3. Durability & Recovery Rules
1. **Monotonicity:** LSNs are strictly monotonic 64-bit byte offsets.
2. **Group Commit:** Waiters are batched within an adaptive window (50µs – 1ms). Acknowledgement is sent only after the corresponding LSN has been `fdatasync`ed to disk.
3. **Recovery Redo:** Replay from the last valid checkpoint to the first record that fails CRC32C. Transactions without a commit record are marked aborted; no undo logging needed under MVCC.
4. **Idempotence:** Replaying the identical WAL stream prefix twice yields identical physical and logical state.
