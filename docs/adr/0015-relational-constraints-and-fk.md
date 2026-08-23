# ADR 0015: Relational Constraints, Foreign Keys & Referential Actions Engine
# Status: ACCEPTED
# Date: 2026-08-20

## Context
A complete relational database must guarantee data integrity through declarative constraints. Without engine-level foreign key and unique constraint enforcement, applications risk orphaned records, duplicate primary keys, and invalid nulls during concurrent operations.

## Decision
We implement `samardb-constraints` (`src/constraints.lpp`) as an integral layer between the SQL execution engine and the physical storage layer:
1. **Pre-WAL Validation**: All constraint checks are executed before WAL log appending. If any constraint is violated, the transaction aborts with zero storage or WAL footprint.
2. **Declarative Actions**: Foreign keys support `ON DELETE RESTRICT`, `ON DELETE CASCADE`, and `ON DELETE SET_NULL`.
3. **Multi-Column Constraints**: CHECK ranges and NOT NULL bitmaps are validated using direct byte/int bitmasks.

## Consequences
- **Positive**: Guarantees strict referential integrity matching PostgreSQL SQL standard behavior.
- **Positive**: Eliminates orphan tuples and corrupt parent-child linkages.
- **Positive**: Zero performance overhead for tables without active foreign key rules.
