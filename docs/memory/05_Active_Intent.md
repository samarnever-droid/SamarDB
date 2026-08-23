# 05 Active Intent — SamarDB

## Current Goal
Hardening, High-Volume Production Load Testing & Adversarial Dark-Corner Chaos Verification (`samardb-adversarial-chaos`).

## Current Status
- **Cumulative Regression & Chaos Suite** (re-measured 2026-08-21): **514 assertions passing, 1 intermittently failing** across all 18 test binaries:
  - `test_skeleton.exe`: 16/16
  - `test_phase1.exe`: 30/30
  - `test_phase2.exe`: 19/19
  - `test_phase3.exe`: 31/31
  - `test_phase4.exe`: 47/47
  - `test_phase5.exe`: 34/34
  - `test_phase6.exe`: 51/51
  - `test_phase7.exe`: 54/54
  - `test_phase8.exe`: 24/24
  - `test_phase9.exe`: 26/26
  - `test_phase10.exe`: 16/16
  - `test_phase11.exe`: 36/36
  - `test_phase12.exe`: 27/27
  - `test_phase13.exe`: 35/35
  - `test_phase14.exe`: **FLAKY — 20/20 on ~75% of runs, otherwise segfault (exit 139) or panic at assertion 8**
  - `test_phase15.exe`: 20/20
  - `test_stress_load.exe`: 6/6 workloads completed
  - `test_father_chaos.exe`: 39/39 dark-corner assertions passing
- **Adversarial Resilience**: Slotted page boundaries (0-byte / 8KB / saturation), B+ tree negative & out-of-range keys, double-fault recovery, write-skew SSI detection, savepoint stack isolation, and memory slicing verified.

## ⚠️ Blocker — upstream L++ ARC bug
`test_phase14` (constraints/FK) is **not** a SamarDB logic defect.
`constraint_validate_unique` is trivially correct; the fault is a use-after-free
in the L++ compiler's ARC release path, triggered by the functional
state-threading idiom `ConstraintManager` uses — a rebuild function that aliases
≥2 `List` fields off its own argument while copy-looping a third, on a struct
originating from a constructor helper. Assigning the result back frees list
bodies the new struct still owns.

Filed upstream with a deterministic (30/30) reduced repro:
- `C:\Users\khati\lpp\SAMARDB_FEATURE_REQUESTS.md` (item **B1**)
- `C:\Users\khati\lpp\tests\arc_multi_field_alias_rebuild.lpp`

The previously recorded "497/497 (100%)" was captured on a lucky run. **Treat
the suite as not-green until B1 lands upstream**; any state-threading rebuild in
SamarDB (~200 sites) is exposed to the same corruption.

## Next Up
Phase 16: Bitmap Index Scan Operator & Multi-Index Query Planner (`samardb-bitmap-scan`).
