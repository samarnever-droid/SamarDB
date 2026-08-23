# SamarDB B+ Tree Page On-Disk Binary Format (v1)

- **Version**: 1
- **Page Size**: 8192 bytes
- **Page Types**:
  - `PAGE_BTREE_LEAF = 2`
  - `PAGE_BTREE_INTERNAL = 3`
- **Endianness**: Little-Endian (LE) for multi-byte values

---

## 1. Page Header (Bytes 0..32)

Every B+ Tree page begins with the standard 32-byte SamarDB Page Header:

| Offset | Field | Type | Description |
|---|---|---|---|
| `0..4` | `checksum` | `u32` | CRC32C of bytes `4..8192`. Verified on every read (Invariant I8). |
| `4..12` | `lsn` | `u64` | Log Sequence Number of last modifying WAL record (Invariant I2). |
| `12..14` | `page_type` | `u16` | `2` for Leaf node, `3` for Internal node. |
| `14..16` | `flags` | `u16` | `0x01` = Root, `0x02` = Leaf, `0x04` = Rightmost node. |
| `16..18` | `format_version` | `u16` | Always `1`. |
| `18..20` | `cell_count` | `u16` | Number of key/value cells (or child pointers) in this page. |
| `20..22` | `lower` | `u16` | Byte offset to end of cell pointer array (grows upward). |
| `22..24` | `upper` | `u16` | Byte offset to start of cell content space (grows downward). |
| `24..28` | `right_sibling` | `u32` | Page ID of right sibling node (`0` if rightmost). Enables B-link tree right-walk. |
| `28..32` | `left_sibling` | `u32` | Page ID of left sibling node (`0` if leftmost). |

---

## 2. Cell Pointer Array (Bytes 32..lower)

Immediately following the 32-byte header is an array of `u16` offsets pointing to cells stored at the bottom of the page (`upper..8192`), kept in **strictly sorted key order**:

| Offset | Field | Type | Description |
|---|---|---|---|
| `32..34` | `cell_offset[0]` | `u16` | Offset within page to smallest key's cell. |
| `34..36` | `cell_offset[1]` | `u16` | Offset to second smallest key's cell. |
| `...` | `...` | `u16` | ... |
| `lower-2..lower` | `cell_offset[N-1]` | `u16` | Offset to largest key's cell. |

---

## 3. Cell Formats

### 3.1 Leaf Node Cell (`page_type = 2`)
Stores key and physical Row ID (`page_id`, `slot_id`):

| Offset | Field | Type | Description |
|---|---|---|---|
| `0..8` | `key` | `i64` | Indexed 64-bit integer key (or 8-byte prefix). |
| `8..12` | `heap_page_id` | `u32` | Target heap table page ID. |
| `12..14` | `heap_slot_id` | `u16` | Target slot index within heap page. |
| `14..16` | `reserved` | `u16` | Reserved for flags (e.g. deleted/tombstone). |

- **Total Leaf Cell Size**: 16 bytes.
- **Maximum Leaf Capacity**: \((8192 - 32) / (16 + 2) \approx 453\) entries per 8KB leaf page.

### 3.2 Internal Node Cell (`page_type = 3`)
Stores key and child page pointer:

| Offset | Field | Type | Description |
|---|---|---|---|
| `0..8` | `key` | `i64` | Lower bound key for child subtree. |
| `8..12` | `child_page_id` | `u32` | Page ID of subtree child node. |
| `12..16` | `reserved` | `u32` | Reserved (padding). |

- **Total Internal Cell Size**: 16 bytes.
- **Maximum Internal Capacity**: \((8192 - 32) / (16 + 2) \approx 453\) child pointers per 8KB page.

---

## 4. Invariants & Split Protocol

1. **Strict Key Ordering**: `cell_offset[i].key < cell_offset[i+1].key` for all \(0 \le i < N-1\).
2. **Binary Search**: Key lookups in internal and leaf nodes execute via \(O(\log N)\) binary search on `cell_offset` array.
3. **Lehman-Yao B-link Protocol**: If a concurrent inserter split the node, search follows `right_sibling` if target `key >= right_sibling.min_key`.
4. **50/50 Node Split**: When free space (`upper - lower < 18`) is exhausted:
   - Allocate new right sibling page `P_right`.
   - Copy upper 50% cells to `P_right`.
   - Link `P_curr.right_sibling = P_right.page_id`.
   - Propagate split key to parent node.
