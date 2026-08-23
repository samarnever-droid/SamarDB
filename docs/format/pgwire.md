# Format Spec: PostgreSQL Frontend/Backend Wire Protocol (`samardb-pgwire`)
# Version: 1
# Reference: https://www.postgresql.org/docs/current/protocol-message-formats.html

## Overview

SamarDB exposes a subset of the PostgreSQL wire protocol (protocol version 3.0) to
allow standard PostgreSQL clients (psql, libpq, pgAdmin, JDBC, etc.) to connect
without modification.

All integers are **big-endian** unless otherwise noted.
All messages prefixed with a type byte are **backend messages** sent to the client.
The **startup message** (client → server) is the only message without a type byte prefix.

---

## 1. Startup Sequence (Client → Server)

### 1.1 Startup Message

Sent once at connection open. No leading type byte.

| Offset | Size | Field         | Value                                         |
|--------|------|---------------|-----------------------------------------------|
| 0      | 4    | `length`      | Total message length in bytes (Int32, BE)     |
| 4      | 4    | `proto_major` | Protocol major version = 3 (Int16, BE)        |
| 6      | 4    | `proto_minor` | Protocol minor version = 0 (Int16, BE)        |
| 8      | …    | `params`      | Key=value pairs, null-terminated strings       |
| …      | 1    | `terminator`  | `\0` byte (end of parameter list)             |

Example params: `"user\0samardb\0database\0samardb\0\0"`

### 1.2 Authentication OK (Server → Client)

| Offset | Size | Field     | Value                     |
|--------|------|-----------|---------------------------|
| 0      | 1    | `type`    | `R` (0x52)                |
| 1      | 4    | `length`  | 8 (Int32, BE)             |
| 5      | 4    | `auth_ok` | 0 (Int32, BE)             |

### 1.3 Backend Key Data (Server → Client)

| Offset | Size | Field     | Value                        |
|--------|------|-----------|------------------------------|
| 0      | 1    | `type`    | `K` (0x4B)                   |
| 1      | 4    | `length`  | 12 (Int32, BE)               |
| 5      | 4    | `pid`     | Backend process ID (Int32)   |
| 9      | 4    | `key`     | Cancel key (Int32)           |

### 1.4 Ready For Query (Server → Client)

Sent after startup and after every command completes.

| Offset | Size | Field    | Value                                |
|--------|------|----------|--------------------------------------|
| 0      | 1    | `type`   | `Z` (0x5A)                          |
| 1      | 4    | `length` | 5 (Int32, BE)                       |
| 5      | 1    | `status` | `I` = Idle, `T` = In Tx, `E` = Err  |

---

## 2. Simple Query Protocol (Client → Server → Client)

### 2.1 Query Message (Client → Server)

| Offset | Size | Field    | Value                            |
|--------|------|----------|----------------------------------|
| 0      | 1    | `type`   | `Q` (0x51)                       |
| 1      | 4    | `length` | 4 + len(sql) + 1 (Int32, BE)    |
| 5      | …    | `sql`    | SQL string, null-terminated      |

### 2.2 Row Description (Server → Client)

Sent once before data rows, describing column names and types.

| Offset | Size | Field       | Value                              |
|--------|------|-------------|------------------------------------|
| 0      | 1    | `type`      | `T` (0x54)                        |
| 1      | 4    | `length`    | Int32, BE                          |
| 5      | 2    | `ncols`     | Number of columns (Int16, BE)      |

Per column (immediately following `ncols`):

| Size | Field         | Value                                    |
|------|---------------|------------------------------------------|
| …    | `col_name`    | Column name, null-terminated string      |
| 4    | `table_oid`   | 0 (not tracked)                          |
| 2    | `col_attnum`  | 0 (not tracked)                          |
| 4    | `type_oid`    | 23 for INT4, 25 for TEXT                 |
| 2    | `type_size`   | -1 for variable, 4 for INT4              |
| 4    | `type_mod`    | -1 (no modifier)                         |
| 2    | `format`      | 0 = text format                          |

### 2.3 Data Row (Server → Client)

Sent once per result tuple.

| Offset | Size | Field    | Value                            |
|--------|------|----------|----------------------------------|
| 0      | 1    | `type`   | `D` (0x44)                      |
| 1      | 4    | `length` | Int32, BE                        |
| 5      | 2    | `ncols`  | Number of column values (Int16, BE) |

Per column value:

| Size | Field     | Value                                            |
|------|-----------|--------------------------------------------------|
| 4    | `col_len` | Length of value in bytes, or -1 for NULL (Int32) |
| …    | `col_val` | Value bytes (text representation)               |

### 2.4 Command Complete (Server → Client)

| Offset | Size | Field    | Value                              |
|--------|------|----------|------------------------------------|
| 0      | 1    | `type`   | `C` (0x43)                        |
| 1      | 4    | `length` | 4 + len(tag) + 1 (Int32, BE)      |
| 5      | …    | `tag`    | Command tag e.g. `SELECT 3\0`, `INSERT 0 1\0` |

### 2.5 Error Response (Server → Client)

| Offset | Size | Field    | Value                              |
|--------|------|----------|------------------------------------|
| 0      | 1    | `type`   | `E` (0x45)                        |
| 1      | 4    | `length` | Int32, BE                          |
| 5      | 1    | `field`  | `S` = Severity, `M` = Message, `0` = terminator |
| …      | …    | `value`  | Field value, null-terminated       |

Minimum error message:  `S\0ERROR\0M\0<message>\0\0`

---

## 3. Version Field

All SamarDB pgwire frames include an internal version byte at offset 0 of the
**SimSocket** buffer header (not visible to the client), used only during
deterministic simulation testing to verify framing correctness.

| Version | Value | Meaning                  |
|---------|-------|--------------------------|
| 1       | 0x01  | Phase 6 initial protocol |
