#!/usr/bin/env bash
# ==============================================================================
# SamarDB v1.2.0 Linux Release Build Script (Powered by L++ v1.2.0)
# ==============================================================================
set -euo pipefail

mkdir -p dist

echo "================================================================"
echo "     Building SamarDB v1.2.0 for Linux (x86_64 & WASM)         "
echo "================================================================"

# 1. Compile Native Linux Server Daemon (PostgreSQL Wire Protocol v3.0 on port 5432)
echo "[*] Compiling Native Linux Server Daemon (AOT ELF Binary)..."
lpp src/main.lpp -o dist/samardb-server
chmod +x dist/samardb-server
echo "    -> Output: dist/samardb-server"

# 2. Compile Serverless WebAssembly Engine (WASI Edge Sandbox)
echo "[*] Compiling Serverless WebAssembly Engine (wasm32-wasi)..."
lpp src/bench_samardb_vs_postgres_full.lpp --backend wasm -o dist/samardb-serverless.wasm
echo "    -> Output: dist/samardb-serverless.wasm"

# 3. Generate SHA-256 Checksums
echo "[*] Computing release cryptographic checksums..."
cd dist
sha256sum samardb-server samardb-serverless.wasm > SHA256SUMS
cat SHA256SUMS
cd ..

echo "================================================================"
echo " SamarDB v1.2.0 Release Packages Built Successfully in /dist    "
echo "================================================================"
