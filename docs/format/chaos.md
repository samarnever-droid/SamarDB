# Format Spec: Chaos Testing & Fault Injection Events (`samardb-chaos`)
# Version: 1

## Overview

SamarDB Phase 10 implements adversarial fault injection to formally verify ACID durability,
crash recovery correctness, self-healing under torn writes, bitrot detection, and distributed
consensus safety under network partitions.

---

## 1. Fault Types

| Code | Fault Name             | Description                                                   | Expected Invariant Behavior                     |
|------|------------------------|---------------------------------------------------------------|-------------------------------------------------|
| `1`  | `FAULT_POWER_LOSS`     | Instant node crash; unflushed page dirty buffers discarded    | Invariant I1: All committed WAL logs survive    |
| `2`  | `FAULT_TORN_WRITE`     | Partial page write (first 4KB written, remaining 4KB zeroed)  | Invariant I8: CRC32C fails; REDO restores page  |
| `3`  | `FAULT_BITROT`         | Single-byte silent flip at offset `K` in an on-disk page      | Invariant I8: Page rejected on read via CRC32C  |
| `4`  | `FAULT_NET_PARTITION`  | Bidirectional packet drop between isolated node groups        | Invariant I1: Minority partition rejects writes |

---

## 2. Fault Injection Event Record

| Field          | Type   | Description                                            |
|----------------|--------|--------------------------------------------------------|
| `fault_type`   | `Int`  | Fault type code (1..4)                                 |
| `target_page`  | `Int`  | Target page ID subjected to corruption                 |
| `byte_offset`  | `Int`  | Offset where corruption was injected                   |
| `detected`     | `Bool` | Whether storage layer detected checksum mismatch       |
| `recovered`    | `Bool` | Whether ARIES REDO successfully reconstructed state    |
