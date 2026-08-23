# Page Format Specification (v1)

Format version: `1`  
Default page size: `8192` bytes (selectable at cluster init: 4096, 8192, 16384, 32768)

---

## 1. Binary Page Header Layout

Every page on disk begins with a 32-byte header:

| Offset | Size (bytes) | Type | Field | Description |
|---|---|---|---|---|
| `0` | `4` | `u32` | `checksum` | CRC32C computed over bytes `[4, page_size)` |
| `4` | `8` | `u64` | `lsn` | LSN of the last WAL record modifying this page |
| `12` | `2` | `u16` | `page_type` | Page type discriminator (see Page Types table) |
| `14` | `2` | `u16` | `flags` | Bitmask flags (overflow, all_visible, compressed, encrypted) |
| `16` | `2` | `u16` | `format_version` | Layout version (must be `1`) |
| `18` | `2` | `u16` | `slot_count` | Number of item slots in the slot array |
| `20` | `2` | `u16` | `lower` | Byte offset pointing to the end of the slot array |
| `22` | `2` | `u16` | `upper` | Byte offset pointing to the start of the unallocated tuple area |
| `24` | `4` | `u32` | `special_offset` | Byte offset to access-method-specific tail area (`page_size` if unused) |
| `28` | `4` | `u32` | `reserved` | Must be zero |

---

## 2. Page Types

| ID | Name | Usage |
|---|---|---|
| `1` | `PAGE_HEAP` | Standard slotted row storage |
| `2` | `PAGE_BTREE_INTERNAL` | B+Tree internal routing node |
| `3` | `PAGE_BTREE_LEAF` | B+Tree leaf node with key-value data |
| `4` | `PAGE_FTS` | Full-text inverted index posting segments |
| `5` | `PAGE_HNSW` | Vector index HNSW graph nodes |
| `6` | `PAGE_META` | Database and relation metadata page |
| `7` | `PAGE_FREESPACE` | Free space map tracking page densities |
| `8` | `PAGE_OVERFLOW` | Chained overflow pages for oversized tuples |

---

## 3. Slotted Heap Page Data Layout

```text
+-----------------------------------------------------------------------+
| Header (32 bytes)                                                     |
+-----------------------------------------------------------------------+
| Slot Array: slot 0, slot 1, ... slot (N-1)                            |
| (Grows downwards from offset 32 towards 'lower')                      |
+-----------------------------------------------------------------------+
| < Unallocated Free Space (between 'lower' and 'upper') >              |
+-----------------------------------------------------------------------+
| Tuple Data & Version Records                                          |
| (Grows upwards from 'page_size' / 'special_offset' towards 'upper')   |
+-----------------------------------------------------------------------+
| Special Access Method Tail Area (optional, [special_offset..size))    |
+-----------------------------------------------------------------------+
```

### Slot Structure (6 bytes each)
```text
off  size  field
0    2     offset   (u16 offset from start of page to tuple)
2    2     length   (u16 byte length of tuple)
4    2     flags    (u16: 0x01=live, 0x02=redirected, 0x04=dead)
```

---

## 4. Invariants Protected
- **I2:** A page on disk is never partially applied: either old image or new image is readable and checksum-valid.
- **I5:** After recovery, no page LSN exceeds the durable WAL LSN.
- **I8:** Every byte read from disk is CRC32C verified before it is trusted.
