# SamarDB Record & Schema Serialization Format (v1)

- **Version**: 1
- **Endianness**: Little-Endian (LE) for multi-byte values
- **Supported Data Types**:
  - `TYPE_INT = 1` (8 bytes, `i64`)
  - `TYPE_VARCHAR = 2` (variable-length UTF-8, `u16` length + bytes)
  - `TYPE_BOOL = 3` (1 byte, `u8`: `1 = true`, `0 = false`)

---

## 1. Relational Tuple Payload Layout

Inside an MVCC Tuple (which has a 24-byte MVCC header: `xmin`, `xmax`, `cid`, `infomask`, `payload_len`), the user payload is formatted as:

| Byte Offset | Field | Type | Description |
|---|---|---|---|
| `0..2` | `col_count` | `u16` | Number of columns in this record. |
| `2..4` | `null_bitmap` | `u16` | Bit `i` is set (1) if column `i` is NULL. |
| `4..4 + (col_count * 2)` | `col_offsets` | `u16[]` | Byte offset relative to start of record for each column's value. |
| `4 + (col_count * 2)..end` | `col_data` | `bytes` | Column values laid out sequentially. |

---

## 2. Column Value Encodings

### 2.1 `TYPE_INT` (8 bytes)
- Stored as 64-bit signed integer (little-endian).
- `bytes_get64le` / `bytes_set64le`.

### 2.2 `TYPE_BOOL` (1 byte)
- `1` = `true`, `0` = `false`.
- `bytes_get8` / `bytes_set8`.

### 2.3 `TYPE_VARCHAR` (2 + N bytes)
- `0..2`: `len` (`u16`, length of string in bytes).
- `2..2+len`: raw ASCII/UTF-8 character bytes.

---

## 3. Catalog & Table Metadata Encoding

Table schemas are represented by column definitions:
- `Column(name: Str, data_type: Int, is_primary_key: Bool, is_nullable: Bool)`
- `TableSchema(table_name: Str, columns: List[Column], root_heap_page_id: Int, root_index_page_id: Int)`
