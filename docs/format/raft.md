# Format Spec: Raft Consensus Messages (`samardb-raft`)
# Version: 1
# Reference: "In Search of an Understandable Consensus Algorithm" (Ongaro & Ousterhout, 2014)

## Overview

All Raft messages are exchanged between `RaftNode` instances through a deterministic
`SimNetwork` (injectable, no OS sockets). Each message is a flat struct serialised as a
fixed-size byte frame for simplicity. All integers are **little-endian** (matching the
host `i64` layout used by the L++ backend).

---

## 1. RaftMessage Header

Every Raft message begins with an 8-byte header:

| Offset | Size | Field        | Value                                |
|--------|------|--------------|--------------------------------------|
| 0      | 1    | `msg_type`   | See types below                      |
| 1      | 1    | `from_node`  | Sender node ID (0–127)               |
| 2      | 1    | `to_node`    | Recipient node ID (0–127)            |
| 3      | 1    | `_pad`       | 0x00 (reserved)                      |
| 4      | 4    | `_pad32`     | 0x00000000 (reserved)                |

### Message Type Constants

| Type         | Code | Description               |
|--------------|------|---------------------------|
| `REQUEST_VOTE`     | 1    | Candidate asks for vote   |
| `VOTE_REPLY`       | 2    | Follower grants/denies vote |
| `APPEND_ENTRIES`   | 3    | Leader replication / heartbeat |
| `APPEND_REPLY`     | 4    | Follower replication ACK  |

---

## 2. RequestVote (type = 1)

Sent by a candidate to all peers to request their vote for a given term.

| Offset | Size | Field             | Value                                     |
|--------|------|-------------------|-------------------------------------------|
| 0      | 8    | header            | See above                                 |
| 8      | 8    | `term`            | Candidate's current term (Int64, LE)      |
| 16     | 8    | `candidate_id`    | Candidate's node ID (Int64, LE)           |
| 24     | 8    | `last_log_index`  | Index of candidate's last log entry       |
| 32     | 8    | `last_log_term`   | Term of candidate's last log entry        |

Total: **40 bytes**

---

## 3. VoteReply (type = 2)

| Offset | Size | Field           | Value                                     |
|--------|------|-----------------|-------------------------------------------|
| 0      | 8    | header          | See above                                 |
| 8      | 8    | `term`          | Voter's current term                      |
| 16     | 1    | `vote_granted`  | 1 if granted, 0 if denied                 |
| 17     | 7    | `_pad`          | 0x00 (alignment padding)                  |

Total: **24 bytes**

---

## 4. AppendEntries (type = 3)

Sent by leader for log replication or as a heartbeat (entries_count = 0).

| Offset | Size | Field             | Value                                     |
|--------|------|-------------------|-------------------------------------------|
| 0      | 8    | header            | See above                                 |
| 8      | 8    | `term`            | Leader's current term                     |
| 16     | 8    | `leader_id`       | Leader's node ID                          |
| 24     | 8    | `prev_log_index`  | Index of log entry just before new ones   |
| 32     | 8    | `prev_log_term`   | Term of `prev_log_index` entry            |
| 40     | 8    | `leader_commit`   | Leader's `commitIndex`                    |
| 48     | 4    | `entries_count`   | Number of log entries appended (Int32, LE)|
| 52     | 4    | `_pad`            | 0x00000000                                |

Each log entry (immediately following, repeated `entries_count` times):

| Offset | Size | Field    | Value                    |
|--------|------|----------|--------------------------|
| +0     | 8    | `term`   | Entry's term             |
| +8     | 8    | `index`  | Entry's log index        |
| +16    | 8    | `data`   | Command payload (Int64)  |

Entry size: **24 bytes**. Maximum `entries_count` in Phase 7: **8**.

Total (heartbeat): **56 bytes**
Total (with N entries): **56 + N × 24 bytes**

---

## 5. AppendReply (type = 4)

| Offset | Size | Field          | Value                                     |
|--------|------|----------------|-------------------------------------------|
| 0      | 8    | header         | See above                                 |
| 8      | 8    | `term`         | Follower's current term                   |
| 16     | 1    | `success`      | 1 if entries accepted, 0 if rejected      |
| 17     | 7    | `_pad`         | 0x00                                      |
| 24     | 8    | `match_index`  | Highest log index known replicated        |

Total: **32 bytes**

---

## 6. SimNetwork Envelope

Messages are enqueued in a `SimNetwork` mailbox rather than sent over TCP.
Each envelope stores:

| Field        | Type   | Description                          |
|--------------|--------|--------------------------------------|
| `to_node`    | Int    | Destination node ID                  |
| `msg_type`   | Int    | Message type code                    |
| `term`       | Int    | Extracted term (for quick sorting)   |
| `payload`    | Bytes  | Full serialised message frame        |

---

## 7. Version Field

`raft.lpp` writes `0x01` at byte 0 of the SimNetwork version header (internal only).
