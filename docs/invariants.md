# SamarDB Correctness and Durability Invariants

This document is the contract of the entire SamarDB project. Every invariant has an ID, an active assertion in code, and tests in the test suite proving the assertion fires when violated.

---

| ID | Invariant | Scope | Verification / Assertion |
|---|---|---|---|
| **I1** | If a commit was acknowledged, its effects survive any crash after that acknowledgment. | Durability | Crash matrix + WAL replay recovery tests |
| **I2** | A page on disk is never partially applied: either the old image or the new image is readable and checksum-valid. | Storage / Pager | CRC32C validation + FPI torn-write recovery |
| **I3** | Every index entry resolves to a heap version that existed at some snapshot. | Access Methods | `samar verify` index-to-heap agreement checker |
| **I4** | No transaction observes a version outside its snapshot's visibility rules. | MVCC / Txn | MVCC visibility unit tests + Elle isolation checker |
| **I5** | WAL LSNs are strictly monotonic, and after recovery no page LSN exceeds the durable LSN. | WAL / Recovery | LSN monotonicity assert + recovery bounds check |
| **I6** | Recovery is idempotent: replaying the same WAL prefix twice yields identical state. | Recovery | Idempotent replay test harness |
| **I7** | A version is reclaimed only when no live snapshot can see it. | Harvester | Epoch-based reclamation visibility assertions |
| **I8** | Every byte read from disk is checksum-verified before it is trusted. | Storage / I/O | CRC32C verification on page and WAL record decodes |
| **I9** | The commit path performs no dynamic allocation and no unbounded wait. | Performance / Safety | Allocation tracker assert zero allocs in commit group |
| **I10** | On any detected invariant violation the process aborts rather than writing suspect data. | Safety | Invariant abort handler (fail-loud) |
| **I11** | *(P7)* A distributed transaction is never partially committed. | Distribution | 2PC recovery + Jepsen/Elle test suites |
| **I12** | *(P7)* Raft log matching and leader completeness hold exactly as stated in the Raft paper. | Consensus | Raft state machine simulator + chaos suites |
