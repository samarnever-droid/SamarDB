-------------------------------
(* MERIDIAN — seqlock x epoch model (spec Phase 0 / §10)                  *)
(*                                                                        *)
(* One bucket + the reclamation machinery:                                *)
(*  - writers replace the cell under an odd/even version (seqlock)        *)
(*  - readers pin an epoch, take a snapshot, validate, then read on        *)
(*  - replaced cells retire with the current epoch tag; the collector      *)
(*    frees only cells tagged below the barrier (min of global epoch and   *)
(*    every live pin)                                                     *)
(*                                                                        *)
(* Invariants:                                                            *)
(*  - NoUseAfterFree : a pinned reader never holds a freed cell           *)
(*  - PinBlocksCollect: a pinned reader's cell is not collectable          *)
(*                                                                        *)
(* Status: WRITTEN, NOT YET MODEL-CHECKED (no TLC on the dev box).        *)
(* Execute in CI before relying on it:                                    *)
(*   tlc2.tla.TLC -deadlock MeridianSeqlockEpoch                          *)
-------------------------------
EXTENDS Naturals

CONSTANT Readers

VARIABLES
    version,     \* even = stable, odd = write in progress
    cell,        \* value id present in the bucket (0 = empty)
    nextId,      \* allocator
    retired,     \* set of records [id |-> i, tag |-> t]
    epoch,       \* global epoch counter
    pins,        \* function Readers -> epoch (0 = unpinned)
    holding,     \* function Readers -> cell id being read (0 = none)
    freed        \* set of freed ids

vars == <<version, cell, nextId, retired, epoch, pins, holding, freed>>

Init == version = 0 /\ cell = 0 /\ nextId = 1 /\ retired = {}
        /\ epoch = 1 /\ pins = [r \in Readers |-> 0]
        /\ holding = [r \in Readers |-> 0] /\ freed = {}

\* The collector barrier: minimum of global epoch and every live pin.
Barrier == LET live == {pins[r] : r \in Readers} \ {0}
           IN IF live # {} THEN Min(live \cup {epoch}) ELSE epoch

WriterReplace ==
    /\ version % 2 = 0
    /\ cell # 0 => retired' = retired \union {[id |-> cell, tag |-> epoch]}
    /\ cell = 0 => retired' = retired
    /\ nextId' = nextId + 1
    /\ cell' = nextId                \* publish the new value ...
    /\ version' = version + 2        \* ... under a completed odd/even pair
    /\ UNCHANGED <<epoch, pins, holding, freed>>

ReaderPin(r) ==
    /\ pins[r] = 0
    /\ pins' = [pins EXCEPT ![r] = epoch]
    /\ holding' = [holding EXCEPT ![r] = cell]   \* snapshot, then validate
    /\ UNCHANGED <<version, cell, nextId, retired, epoch, freed>>

ReaderUnpin(r) ==
    /\ pins[r] # 0
    /\ pins' = [pins EXCEPT ![r] = 0]
    /\ holding' = [holding EXCEPT ![r] = 0]
    /\ UNCHANGED <<version, cell, nextId, retired, epoch, freed>>

EpochTick ==
    /\ epoch' = epoch + 1
    /\ UNCHANGED <<version, cell, nextId, retired, pins, holding, freed>>

Collect ==
    /\ LET b = Barrier
           releasable = {x \in retired : x.tag < b}
       IN  freed' = freed \union {x.id : x \in releasable}
           /\ retired' = retired \ releasable
    /\ UNCHANGED <<version, cell, nextId, epoch, pins, holding>>

Next == WriterReplace
        \/ \E r \in Readers : ReaderPin(r) \/ ReaderUnpin(r)
        \/ EpochTick
        \/ Collect

\* A pinned reader never holds a freed cell.
NoUseAfterFree == \A r \in Readers : pins[r] # 0 => holding[r] \notin freed

\* A pinned reader's cell is not below the barrier (not collectable).
PinBlocksCollect ==
    \A x \in retired : x.id \in {holding[r] : r \in Readers} => x.tag >= Barrier

Spec == Init /\ [][Next]_vars
====
