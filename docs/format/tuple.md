# SamarDB On-Disk Format: Slotted Heap Tuple with MVCC Header

**Status:** Standard / Active  
**Module:** `samardb-heap`, `samardb-mvcc`  
**Version:** 1  

## 1. Overview
Every tuple stored in a SamarDB heap page begins with a fixed 24-byte MVCC Tuple Header, followed immediately by user column payload data. This header stores multi-version concurrency control metadata matching PostgreSQL semantics, supporting Snapshot Isolation, non-blocking readers, and background VACUUM reclamation.

---

## 2. Tuple Layout

```
+---------------------------------------------------------------+
|                       Tuple Header (24B)                      |
|  +---------------------------------------------------------+  |
|  | xmin: u64  (8 bytes) - Transaction ID that created tuple |  |
|  | xmax: u64  (8 bytes) - Transaction ID that deleted tuple |  |
|  | cid:  u32  (4 bytes) - Command ID within creating txn    |  |
|  | infomask: u16 (2 bytes) - Status & Visibility Flags     |  |
|  | payload_len: u16 (2 bytes) - User Payload Byte Length    |  |
|  +---------------------------------------------------------+  |
|                     User Payload (0..N bytes)                 |
+---------------------------------------------------------------+
```

---

## 3. Byte-Level Field Specification

| Offset (Bytes) | Field Name | Data Type | Description |
|---|---|---|---|
| `0..7` | `xmin` | `uint64_le` | Transaction ID of creating transaction. |
| `8..15` | `xmax` | `uint64_le` | Transaction ID of deleting/updating transaction (`0` if alive). |
| `16..19` | `cid` | `uint32_le` | Command ID within transaction (for intra-transaction visibility). |
| `20..21` | `infomask` | `uint16_le` | Bit flags for fast-path visibility (see §4). |
| `22..23` | `payload_len` | `uint16_le` | Length of user column payload in bytes. |
| `24..N` | `payload` | `bytes` | Raw user column data. |

---

## 4. Infomask Bit Flags

| Flag Value | Constant | Meaning |
|---|---|---|
| `0x0001` | `HEAP_XMIN_COMMITTED` | `xmin` is known committed (hints CLOG lookup). |
| `0x0002` | `HEAP_XMIN_INVALID` | `xmin` aborted or invalid. |
| `0x0004` | `HEAP_XMAX_COMMITTED` | `xmax` is known committed. |
| `0x0008` | `HEAP_XMAX_INVALID` | `xmax` aborted or invalid. |
| `0x0010` | `HEAP_HOT_UPDATED` | Tuple was updated via Heap-Only Tuple (HOT). |
| `0x0020` | `HEAP_MOVED_OFF_PAGE` | Tuple moved to overflow / toast storage. |

---

## 5. Invariants
1. **Never Overwrite In-Place**: Updates create a new tuple version with `new_tuple.xmin = txn_id` and set `old_tuple.xmax = txn_id`.
2. **Deterministic Sizing**: A tuple occupying slot `i` requires `24 + payload_len` contiguous bytes in the heap page.
3. **Alignment**: Tuple offsets are aligned to 8-byte boundaries within the page.
