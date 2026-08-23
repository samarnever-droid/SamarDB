# SamarDB On-Disk Format: Transaction Status Log (CLOG)

**Status:** Standard / Active  
**Module:** `samardb-tx`  
**Version:** 1  

## 1. Overview
The Transaction Status Log (`clog`) records the commit status of all transactions in SamarDB. Each transaction state is represented by 2 bits, enabling 4 transactions per byte and 32,768 transactions per 8KB page.

---

## 2. Transaction States (2-bit values)

| Binary Value | Decimal Value | State Constant | Description |
|---|---|---|---|
| `00` | `0` | `TXN_IN_PROGRESS` | Transaction is active and has not committed or aborted. |
| `01` | `1` | `TXN_COMMITTED` | Transaction committed successfully and WAL record was flushed. |
| `10` | `2` | `TXN_ABORTED` | Transaction rolled back or crashed before commit. |
| `11` | `3` | `TXN_SUBCOMMITTED` | Reserved for nested savepoints/subtransactions. |

---

## 3. Addressing Math
For a 64-bit Transaction ID `txn_id`:
- `byte_offset = (txn_id / 4)`
- `bit_shift = (txn_id % 4) * 2`
- `state = (byte_val >> bit_shift) & 0x03`

---

## 4. Invariants
1. **Durability**: A transaction transitions to `TXN_COMMITTED` in CLOG only after its commit WAL record has been flushed to durable storage with `fsync`.
2. **Crash Recovery Init**: All transactions marked `TXN_IN_PROGRESS` at the time of a crash are automatically transitioned to `TXN_ABORTED` during the recovery Analysis pass.
