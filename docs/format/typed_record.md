# Format Spec: Typed Variable-Length Records & Multi-Level Index (`samardb-schema`, `samardb-btree`)
# Version: 1

## Overview

SamarDB Phase 12 specifies the byte-level format for typed variable-length relational records
(supporting `INT`, `VARCHAR`, `BOOL`, and `NULL`), and the multi-level internal node page format
for Lehman-Yao B-link tree traversal.

---

## 1. Typed Row Binary Format

Each typed record encodes fixed and variable-length columns with a null bitmap and offset table:

```text
+----------------+----------------+-------------------------------+--------------------------+
| col_count (2B) | null_mask (2B) | offset_table (2B * col_count) | data_payload (variable)  |
+----------------+----------------+-------------------------------+--------------------------+
```

| Field          | Type     | Offset                  | Length              | Description                                       |
|----------------|----------|-------------------------|---------------------|---------------------------------------------------|
| `col_count`    | `UInt16` | `0`                     | `2`                 | Total columns in record                           |
| `null_mask`    | `UInt16` | `2`                     | `2`                 | Bitmask where bit `i=1` denotes column `i` is NULL|
| `offset_table` | `UInt16` | `4`                     | `2 * col_count`     | Byte offset of each column from start of payload  |
| `data_payload` | `Bytes`  | `4 + 2*col_count`       | Variable            | Packed data bytes for non-null columns            |

### Column Data Encodings
- **`TYPE_INT` (1)**: 8-byte Little-Endian 64-bit integer (`Int64`).
- **`TYPE_VARCHAR` (2)**: 2-byte Length prefix (`UInt16`) followed by ASCII character bytes.
- **`TYPE_BOOL` (3)**: 1-byte boolean (`0x00 = false`, `0x01 = true`).
- **`NULL`**: Stored as bit in `null_mask`, 0 bytes in `data_payload`.

---

## 2. Multi-Level B+ Tree Page Header & Child Routing

```text
+-----------------------+---------------------+-------------------+-------------------+
| PageHeader (32 bytes) | right_sibling (4B)  | cell_count (2B)   | cell_ptrs (2B*N)  |
+-----------------------+---------------------+-------------------+-------------------+
```

- **Internal Node Cell**: `[child_page_id: Int32 (4B) | key: Int64 (8B)]` (12 bytes per cell).
- **Routing Invariant**: If `key < cell[0].key`, traverse to `left_child`. If `cell[i].key <= key < cell[i+1].key`, traverse to `cell[i].child_page_id`.
