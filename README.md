# SamarDB v1.2.0 (Linux & Serverless Release)

**SamarDB** is an ultra-high performance distributed relational database engine written in **L++ (Pure Native AOT)**. It natively speaks the **PostgreSQL Wire Protocol v3.0** on port `5432`, providing a lightweight, memory-efficient alternative to traditional PostgreSQL.

[![Release](https://img.shields.io/badge/release-v1.2.0-blue.svg)](https://github.com/samarnever-droid/SamarDB/releases)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Compiler](https://img.shields.io/badge/L%2B%2B-v1.2.0-green.svg)](https://github.com/samarnever-droid/lplusplus)
[![PostgreSQL](https://img.shields.io/badge/pgwire-v3.0_compatible-336791.svg)](docs/adr/0007-network-protocol.md)

---

## ⚡ Key Benchmarks (SamarDB v1.2.0 vs. PostgreSQL 16)

| Metric | PostgreSQL 16 | SamarDB Serverless (WASM) | SamarDB Native Linux Server |
| :--- | :--- | :--- | :--- |
| **Binary Size** | `~48.5 MB` | **`106.94 KB`** (450x smaller) | **`1.08 MB`** (45x smaller) |
| **Idle Memory / RAM** | `30 MB – 128 MB` | **`< 1 MB`** | **`~2.5 MB`** |
| **Direct Cold Connect** | `~2,500 req/s` | `7,352 req/s` | **`24,630 req/s`** (10x faster) |
| **Connection Pooled QPS** | `~42,000 req/s` | **`72,727 req/s`** | `58,139 req/s` |
| **Analytical Hash Joins** | `~11,000 joins/s` | **`30,303 joins/s`** | `10,752 joins/s` |
| **128-Dim AI Vector Search**| `~60,000 vec/s` | **`1,612,903 vec/s`** (SQ8) | `93,984 vec/sec` |
| **500MB DB On-Disk Size** | `~1,074 MB` (1.07 GB) | **`174 MB`** (84% less disk) | **`270 MB`** (75% less disk) |

---

## 🚀 Quick Start on Linux (Ubuntu 22.04 / 24.04)

### 1. Build and Run Directly
```bash
# Clone and build
git clone https://github.com/samarnever-droid/SamarDB.git
cd SamarDB
bash scripts/build_linux_release.sh

# Run native server daemon on port 5432
./dist/samardb-server --port 5432 --data-dir /var/lib/samardb/data
```

### 2. One-Click Linux Systemd Service Install
```bash
bash scripts/install_samardb.sh
```

### 3. Connect with any PostgreSQL Client (`psql`, Prisma, Drizzle, Supabase, instancez)
```bash
psql -h 127.0.0.1 -p 5432 -U samardb -d samardb
```

---

## 🌐 Serverless WebAssembly Deployment

SamarDB compiles directly to a **`106 KB` standalone WASM binary** for Cloudflare Workers, Vercel Edge, and browser runtimes:

```bash
lpp src/server.lpp --backend wasm -o dist/samardb-serverless.wasm
```

---

## 📄 License
Licensed under the [Apache License, Version 2.0](LICENSE) © 2026 SamarDB Authors.
