# ADR 0001: L++ Compiler, Runtime, and Stdlib Readiness Assessment

## Status
Accepted

## Context
SamarDB is a relational database engine written in L++. To achieve PostgreSQL-grade durability, correctness, and performance with low footprint, SamarDB requires specific primitives in the L++ compiler, runtime, and standard library.

This assessment inspects the actual L++ compiler codebase (`C:\Users\khati\lpp`) against the 15 Phase 0 enablement items defined in the SamarDB specification.

---

## Findings by Area

### 1. Byte Buffers & Memory Layout (`buf_*`)
- **Current State:**
  - Runtime implementation in `runtime/lpp_buf.c` and `runtime/windows_x86_64_min.c:441-455`.
  - Builtins defined in `src/builtins.rs:1928-2241`: `buf_alloc`, `buf_free`, `buf_len`, `buf_get8`, `buf_set8`, `buf_get16le`, `buf_set16le`, `buf_get32le`, `buf_set32le`, `buf_copy`, `buf_crc32`, `buf_read`, `buf_write`, `buf_read_str`, `buf_write_str`.
  - Buffer allocation uses OS virtual allocation (`VirtualAlloc` on Windows, `mmap` on Linux) with a leading 8-byte length header.
- **Gaps for SamarDB:**
  - Builtins expose raw integer handles (`Int`), lacking RAII lifecycle management, type-safe slice views, and bounds-checked sub-slices.
  - Missing 64-bit integer accessors (`buf_get64le`, `buf_set64le`, `buf_get64be`, `buf_set64be`) and big-endian 16/32-bit accessors required for lexicographical index keys and 64-bit LSN/TxnId encoding.
- **Action Needed (P0.1, P0.3):**
  - Implement native `Bytes` struct/type in L++ wrapping the `buf_*` primitives with deterministic ARC/destructor lifecycle and slice capabilities.
  - Implement `buf_get64le`/`be` and `buf_set64le`/`be` in compiler builtins and runtime shims.

---

### 2. Type System & Fixed-Width Numerics
- **Current State:**
  - Type definitions in `src/frontend/ast.rs:1-19`, `src/types.rs`, and `src/analysis/type_facts.rs:24-31`.
  - Primitives: `Int` (i64), `Float` (f64), `Bool`, `Char`, `Str`, `Tuple[2..=4]`, `StrSlice`, `Slice[T]`, `Task[T]`.
- **Gaps for SamarDB:**
  - No native fixed-width scalar types (`u8`, `u16`, `u32`, `u64`, `i8`, `i16`, `i32`, `i64`).
  - No `Result[T, E]` algebraic sum type for exhaustive error handling without aborting sessions.
  - Struct layout in `src/analysis/layout.rs:27-45` aligns dynamically to 8 bytes, lacking `repr(packed)` and `align(n)` attributes.
- **Action Needed (P0.2, P0.10, P0.11):**
  - Add fixed-width integer types with explicit truncation/overflow semantics to `ast.rs`, `types.rs`, `type_facts.rs`, and Cranelift/LLVM backends.
  - Add `Result[T, E]` enum with pattern matching exhaustiveness checks.
  - Add layout attributes for packed disk format mapping.

---

### 3. Positional & Durable I/O
- **Current State:**
  - I/O builtins in `src/builtins.rs` and `runtime/windows_x86_64_min.c` only support whole-file reads/writes (`file_read`, `file_write`, `buf_read`, `buf_write`).
- **Gaps for SamarDB:**
  - No positional reads/writes (`pread`, `pwrite`).
  - No explicit durability sync primitives (`fsync`, `fdatasync`, `ftruncate`, `file_open` with flags like `O_DIRECT`, `FILE_FLAG_NO_BUFFERING`).
- **Action Needed (P0.4):**
  - Implement `file_open`, `file_pread`, `file_pwrite`, `file_fsync`, `file_fdatasync`, `file_ftruncate`, and `file_size` builtins with strict error codes.
  - Ensure positional I/O and sync operations are treated as opaque barriers in MIR optimization passes.

---

### 4. ARC Memory Model & Concurrency Passes
- **Current State:**
  - Whole-program single-thread proof in `src/mir/pass_arc_local.rs:1-163` rewrites atomic refcount operations to non-atomic when no thread/spawn is present.
  - Move-out analysis in `src/mir/pass_moveout.rs:1-425` elides retain/release pairs when ownership transfers cleanly across thread boundaries.
  - Static cycle breaking in `src/analysis/cyclebreak.rs:1-645` breaks struct ownership cycles into non-owning edges at compile time.
- **Gaps for SamarDB:**
  - Atomics with explicit memory orderings (`atomic_load`, `atomic_store`, `atomic_add`, `atomic_cas` on `u32`/`u64`) are not yet exposed as user-facing builtins.
  - Multi-executor thread-per-core scheduler (`runtime_start(n)`, `spawn_on(core)`) is not yet implemented (current executor is single-threaded cooperative in `runtime/windows_x86_64_min.c:225-230`).
- **Action Needed (P0.6, P0.7):**
  - Expose atomic builtins with `acquire`, `release`, and `seq_cst` semantics.
  - Build thread-per-core multi-executor runtime in Phase 0.7.

---

### 5. Networking & Sockets
- **Current State:**
  - Async networking runtime in `runtime/lpp-net/` and `src/builtins.rs:2500-2900`.
  - Builtins: `net_listen`, `net_accept`, `net_connect`, `net_recv`, `net_send`, `net_close`.
- **Gaps for SamarDB:**
  - `net_recv` allocates a new `Str` per read, introducing allocation overhead on hot network loops.
  - Lacks zero-copy `net_recv_into(fd, Bytes, off, len)` and unified readiness/completion poller (`io_uring`/`kqueue`/`IOCP`).
- **Action Needed (P0.8, P0.9):**
  - Add zero-copy buffer receive/send builtins.

---

## Implementation Roadmap for Phase 0

| Item | Target | Focus | Risk Level |
|---|---|---|---|
| **P0.1** | `Bytes` Type | Safe byte-buffer container with ARC & slicing | Low |
| **P0.2** | Fixed-Width Integers | `u8`..`u64`, `i8`..`i64` with clean ABI mapping | Low-Med |
| **P0.3** | 64-bit & Endian Codecs | `buf_get64le`/`be`, `buf_set64le`/`be` | Low |
| **P0.4** | Positional Durable File I/O | `file_open`, `pread`, `pwrite`, `fsync`, `fdatasync` | Low-Med |
| **P0.5-P0.15** | Advanced Runtime & SIMD | Poller, Atomics, Multi-executor, SIMD vectors | Med-High |

---

## Decision & Next Steps
1. Land **P0.1 (`Bytes`)**, **P0.2 (Fixed-width integers)**, **P0.3 (64-bit & Endian codecs)**, and **P0.4 (Positional durable file I/O)** first.
2. Formally document on-disk formats: `docs/format/page.md` and `docs/format/wal.md`.
3. Construct deterministic simulator framework (`samardb-io`) before writing the storage pager.
