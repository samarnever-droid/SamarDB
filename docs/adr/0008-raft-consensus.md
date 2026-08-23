# ADR 0008 — Raft Consensus Engine (`samardb-raft`)

## Status
Accepted — Phase 7

## Context

SamarDB now has a fully functional single-node storage engine (Phases 0–6). To
support replication and high availability, we need distributed consensus. Raft was
chosen over Paxos for its understandability, well-defined leader election, and
clear log-matching invariant.

## Decisions

### 1. Deterministic SimNetwork (No OS Sockets)

Following the same pattern as `SimDisk` (Phase 1) and `SimSocket` (Phase 6):

```
SimNetwork:
    mailboxes:  List[List[Int]]  # mailbox per node: queued message indices
    payloads:   List[Bytes]      # indexed message payloads
    msg_count:  Int              # total messages enqueued (ever)
    dropped:    List[Int]        # indices of messages to silently drop (fault injection)
```

No OS threads, no clocks, no real network. The simulation is driven by:
- `simnet_send(net, to_node, msg_bytes)` — enqueue a message
- `simnet_recv(net, node_id)` — dequeue the next message for a node
- `simnet_drop(net, msg_idx)` — mark a future message as dropped (chaos testing)
- `simnet_tick(net, node, msg)` — deliver one queued message to a node

### 2. RaftNode State

```
struct RaftNode:
    node_id:       Int           # node identity (0..N-1)
    role:          Int           # FOLLOWER=0, CANDIDATE=1, LEADER=2
    current_term:  Int           # monotonically increasing
    voted_for:     Int           # node_id we voted for this term, or -1
    vote_count:    Int           # votes received as candidate this term
    commit_index:  Int           # highest log index known committed
    last_applied:  Int           # highest log index applied to state machine
    next_index:    List[Int]     # per-peer: next log index to send (leader only)
    match_index:   List[Int]     # per-peer: highest replicated index (leader only)
    log_terms:     List[Int]     # term of each log entry (parallel list)
    log_data:      List[Int]     # command payload of each log entry
    n_peers:       Int           # total cluster size
```

### 3. Role Constants

```
RAFT_FOLLOWER  = 0
RAFT_CANDIDATE = 1
RAFT_LEADER    = 2
```

### 4. Election Protocol

A node calls `raft_tick_election(node)` when its logical election timer fires:

1. Transition `FOLLOWER → CANDIDATE`
2. Increment `current_term`
3. Record `voted_for = node_id` (vote for self), set `vote_count = 1`
4. Return a `RequestVote` message to be broadcast to all peers

A node processes `RequestVote` via `raft_handle_request_vote(node, msg)`:
- If `msg.term < current_term`: deny
- If `voted_for != -1 && voted_for != msg.candidate_id`: deny
- If `msg.last_log_index < len(log) - 1` or `msg.last_log_term < log_terms[last]`: deny
- Otherwise: grant vote, reset `voted_for`, update term

A node processes `VoteReply` via `raft_handle_vote_reply(node, msg)`:
- If `msg.term > current_term`: revert to follower
- If `msg.vote_granted && role == CANDIDATE`: increment `vote_count`
- If `vote_count > n_peers / 2`: transition to LEADER, send initial heartbeat

### 5. Log Replication Protocol

Leader calls `raft_tick_heartbeat(node)` when its heartbeat timer fires:
- Sends `AppendEntries` (possibly empty = heartbeat) to each follower

`raft_append_entries(node, entries_data)` for client commands:
- Append to local log with `current_term`
- Send `AppendEntries` with new entries to all followers

`raft_handle_append_entries(node, msg)`:
- If `msg.term < current_term`: reject
- If `prev_log_index` not matched: reject with `match_index = last_known`
- Accept entries, advance `commit_index` if `leader_commit > commit_index`
- Return `AppendReply(success=true, match_index=last_appended)`

`raft_handle_append_reply(node, msg)`:
- If `success`: update `match_index[peer]`, advance `commit_index` if quorum reached
- If `!success`: decrement `next_index[peer]` and retry

### 6. Safety Invariants Preserved

| Invariant          | How enforced                                                  |
|--------------------|---------------------------------------------------------------|
| Leader completeness| Only a node with up-to-date log can win election              |
| Log matching       | `prev_log_index` / `prev_log_term` check before accepting     |
| Election safety    | `voted_for` persisted per term; node votes at most once       |
| Quorum commit      | Entry committed only when stored on `> n_peers/2` nodes       |
| Monotone terms     | Any message with higher term forces revert to follower        |

### 7. Scope (Phase 7)

Phase 7 covers election and single-entry replication with up to 3 nodes, driven
entirely through `SimNetwork` without real time. Phase 8 will add serializable
snapshot isolation. Phase 9 will wire Raft into the storage engine.

## Consequences

- All consensus logic is fully deterministic and testable without concurrency.
- Fault injection (dropped messages, partitions) is trivial via `simnet_drop`.
- The SimNetwork seam can be replaced with a real TCP socket layer for production.
