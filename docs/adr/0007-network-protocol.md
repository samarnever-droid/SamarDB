# ADR 0007 — PostgreSQL Wire Protocol Frontend (`samardb-pgwire`)

## Status
Accepted — Phase 6

## Context

SamarDB has a complete storage, MVCC, indexing, SQL parsing, execution, and
cost-based optimizer stack. To be useful to real workloads, it needs a network
interface. Rather than invent a proprietary protocol, SamarDB implements a subset
of the PostgreSQL protocol v3.0 wire format.

This means psql, pgAdmin, JDBC, and any libpq-based client can connect to SamarDB
without modification, treating it as a PostgreSQL server.

## Decision

### 1. Deterministic Injectable Socket Interface (`SimSocket`)

Following the same pattern as `SimDisk` for storage:

```
SimSocket:
    in_buf:   Buf     # bytes written by the client (readable by server)
    out_buf:  Buf     # bytes written by the server (readable by client)
    in_pos:   Int     # read cursor into in_buf
    connected: Bool   # whether the socket is "open"
```

All socket reads and writes go through `simsock_*` functions — no OS network calls.
In production, the same session loop drives a real TCP socket; the injection seam is
at `SimSocket`. This upholds the "no module touches the network directly" invariant.

### 2. Message Framing

Every backend message (server → client) has the standard PostgreSQL prefix:

```
[type: u8][length: i32-BE][ payload... ]
```

The length field includes itself (4 bytes) but not the type byte — exactly matching
the PostgreSQL wire protocol. Framing functions:

- `pgwire_write_byte(sock, b)` — append a single byte to `out_buf`
- `pgwire_write_int32_be(sock, v)` — append 4-byte big-endian integer
- `pgwire_write_int16_be(sock, v)` — append 2-byte big-endian integer
- `pgwire_write_str(sock, s)` — append null-terminated string
- `pgwire_read_byte(sock)` — read one byte from `in_buf`
- `pgwire_read_int32_be(sock)` — read 4-byte big-endian integer from `in_buf`
- `pgwire_read_str(sock)` — read null-terminated string from `in_buf`

### 3. Session State Machine

```
STARTUP → AUTH_OK → READY → (QUERY_RECEIVED → PROCESS_QUERY → READY)*
```

Each state transition sends the appropriate message frame:

- `STARTUP`: read client startup message; validate protocol version 3.0; send `AuthenticationOK`, `BackendKeyData`, `ReadyForQuery`.
- `QUERY_RECEIVED`: read client `Q` message; parse SQL string.
- `PROCESS_QUERY`: call SQL parser → optimizer → executor; stream `RowDescription` + `DataRow` × N + `CommandComplete`; on error, send `ErrorResponse`.
- `READY`: send `ReadyForQuery('I')` and wait.

### 4. Supported Message Types (Phase 6 Scope)

| Direction       | Type | Message           |
|-----------------|------|-------------------|
| Client → Server | —    | StartupMessage    |
| Client → Server | `Q`  | Query             |
| Client → Server | `X`  | Terminate         |
| Server → Client | `R`  | AuthenticationOK  |
| Server → Client | `K`  | BackendKeyData    |
| Server → Client | `Z`  | ReadyForQuery     |
| Server → Client | `T`  | RowDescription    |
| Server → Client | `D`  | DataRow           |
| Server → Client | `C`  | CommandComplete   |
| Server → Client | `E`  | ErrorResponse     |

### 5. Column Type OIDs

| L++ Column Type | PostgreSQL OID | Name    |
|-----------------|----------------|---------|
| `TYPE_INT`      | 23             | `int4`  |
| `TYPE_VARCHAR`  | 25             | `text`  |
| `TYPE_BOOL`     | 16             | `bool`  |

All values are sent as text format (format code `0`). Binary format (`1`) is
deferred to Phase 7.

### 6. Error Handling

Every unrecognized query, parser error, or execution failure produces an
`ErrorResponse` with at minimum:

- Field `S` (Severity): `"ERROR"`
- Field `M` (Message): human-readable error string
- Terminator: `\0`

After an `ErrorResponse`, a `ReadyForQuery('I')` (Idle state) is always sent to
keep the client session alive.

### 7. Startup Validation

- Check `proto_major == 3`. If not, send `ErrorResponse` and close.
- Parse `user` and `database` params; log but do not authenticate in Phase 6
  (deferred to Phase 7 with md5/SCRAM-SHA-256).

## Consequences

- Any libpq-compatible client (psql, JDBC, pgAdmin) can connect without modification.
- The deterministic `SimSocket` allows the full session protocol to be tested in
  isolation from OS networking in the test suite.
- Text-only output is correct and universally supported; binary format can be added
  in Phase 7 without breaking existing clients (clients negotiate format).
