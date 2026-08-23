# SamarDB

**SamarDB** is a pure native, distributed relational database engine written in **L++ (Pure Native AOT)**.

---

## ⚡ Core Architecture

- **Storage Engine:** Slotted Page Heap storage with 32-byte page headers, CRC32C per-page checksums, and dynamic page compaction.
- **Indexing:** High-concurrency Lehman-Yao B+ Tree index with latch-free read traversing and right-sibling pointer link traversal.
- **Concurrency Control:** Cahill Serializable Snapshot Isolation (SSI) detecting dangerous structures ($\text{T}_1 \xrightarrow{rw} \text{T}_2 \xrightarrow{rw} \text{T}_3$) and preventing write skew without heavy table locks.
- **Durability & Recovery:** Strict ARIES Write-Ahead Logging (WAL) with physiological logging, checkpointing, and idempotent REDO / rollback UNDO recovery passes.
- **Distributed Consensus:** Multi-shard Raft consensus replicated state machine supporting term leader elections, majority quorum validation, and partition healing.
- **Query Engine:** Cost-Based Optimizer (CBO), vectorized execution engine, analytical hash aggregations, and hash joins.
- **Network Interface:** PostgreSQL v3.0 Wire Protocol (`samardb-pgwire`) frontend compatible with libpq, psql, JDBC, and standard PostgreSQL drivers.

---

## 🛡️ Production Qualification & CI/CD Matrix

SamarDB features a 5-pillar CI/CD verification matrix:

1. **`release.yml`**: Cross-platform standalone database binary builds for Windows, Linux, and macOS.
2. **`workload-5000-quadrant.yml`**: 5,000 continuous quadrant requests (Writes, Point Reads, In-place Updates, Compaction, B+Tree Scans, SIMD Filtering, Analytical Aggregations, Cahill SSI, Raft, ARIES WAL).
3. **`sharding-cluster.yml`**: 9-node 3-shard distributed Raft cluster with live leader crash injection and split-brain network partition torture.
4. **`chaos-crash-recovery.yml`**: 39-assertion adversarial "Father of Edge Cases" chaos suite, torn page detection, and double-fault crash recovery.
5. **`final-boss-30min-soak.yml`**: 30-minute endurance soak qualification testing non-stop sustained transactional workloads under concurrent failovers.

---

## 🚀 Building & Running

```bash
# Compile any test or engine binary using L++
lpp src/test_5000_quadrant.lpp
./src/test_5000_quadrant.exe

# Run Distributed 9-Node 3-Shard Raft Cluster Torture
lpp src/test_cluster_raft.lpp
./src/test_cluster_raft.exe

# Run Adversarial Chaos Suite
lpp src/test_father_chaos.lpp
./src/test_father_chaos.exe
```
