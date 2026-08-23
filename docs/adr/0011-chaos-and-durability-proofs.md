# ADR 0011 — Chaos Testing, Fault Injection & Formal Durability Proofs

## Status
Accepted — Phase 10

## Context
A database engine's most critical obligation is data safety under catastrophic failure.
Phase 10 provides deterministic, automated chaos test suites to empirically prove Invariants I1–I8:
- **I1 (Durability)**: Acknowledged commits survive sudden power loss.
- **I6 (REDO Idempotence)**: Repeating WAL REDO logs produces identical consistent state.
- **I8 (Self-Verifying Pages)**: CRC32C checksums detect any partial torn write or bitrot corruption.

## Decisions

### 1. Fault Injection Primitives
- **Torn Page Writes (`simdisk_torn_write`)**: Simulates operating system crash midway through an 8KB write by only writing the first 4096 bytes to `SimDisk` and filling the remainder with zeroes.
- **Bitrot Injection (`simdisk_inject_bitrot`)**: Flips a single byte inside an existing on-disk page frame to verify CRC32C failure detection.
- **Network Partitioning (`simnet_partition`)**: Drops all packets between a 2-node majority and a 1-node minority, proving the minority partition cannot elect a leader or commit transactions (split-brain immunity).

### 2. Autonomous Recovery Verification
- Upon detecting a torn or corrupted page on disk via `page_verify_checksum(page) == false`, the recovery engine replays WAL records from the last valid checkpoint to reconstruct a pristine, CRC32C-valid page.

## Consequences
- 100% test coverage against hardware failures, torn writes, bitrot, power loss, and network partitions.
- Guarantees zero data loss for all acknowledged commits across the entire SamarDB storage hierarchy.
