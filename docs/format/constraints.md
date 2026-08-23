# SamarDB Constraints, Foreign Keys & Referential Integrity Specification
# File: docs/format/constraints.md
# Version: 1

## Overview

SamarDB enforces relational integrity at the engine level prior to WAL logging and heap mutation.
This specification defines the data structures and state machines for NOT NULL, CHECK, UNIQUE,
and FOREIGN KEY referential constraints with cascading actions.

---

## 1. Constraint Categories & Codes

| Constraint Type | Code | Description |
| :--- | :---: | :--- |
| `NOT_NULL` | 1 | Disallows null values for mandatory columns. |
| `CHECK_RANGE` | 2 | Asserts numeric values fall within `[min_val, max_val]`. |
| `UNIQUE` | 3 | Guarantees distinct values via primary/secondary B+ Tree index. |
| `FOREIGN_KEY` | 4 | Asserts referencing key exists in referenced parent relation. |

---

## 2. Foreign Key Referential Actions (`ON DELETE`)

| Action Code | Action Name | Execution Behavior |
| :---: | :--- | :--- |
| 1 | `RESTRICT` | Rejects parent deletion if matching child rows exist. |
| 2 | `CASCADE` | Automatically deletes matching child rows when parent is deleted. |
| 3 | `SET_NULL` | Sets referencing foreign key columns to NULL on parent deletion. |

---

## 3. Enforcement Order (Pre-Flight Pipeline)

```text
Incoming DML Tuple
  ↓
1. NOT NULL validation (fail-fast)
  ↓
2. CHECK range validation (fail-fast)
  ↓
3. UNIQUE / Primary Key index lookup (fail-fast on collision)
  ↓
4. FOREIGN KEY parent existence check (fail-fast if missing)
  ↓
WAL Logging (LSN generation)
  ↓
Slotted Page Heap Mutation
```
