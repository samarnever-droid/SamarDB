# ADR 0014 — Multi-Table System Catalog, DDL Engine & Schema Evolution

## Status
Accepted — Phase 13

## Context
Up to Phase 12, table and index descriptors were passed explicitly as local struct instances.
A full-featured DBMS requires a centralized **System Catalog** that dynamically registers tables,
columns, and indexes, enabling runtime DDL execution (`CREATE TABLE`, `DROP TABLE`, `CREATE INDEX`,
`DROP INDEX`, `ALTER TABLE ADD COLUMN`) and catalog-driven query optimization.

## Decisions

### 1. Catalog Architecture (`SystemCatalog`)
- Maintained as an active system catalog tracking:
  - Tables: Table ID, Table Name, Heap Root Page, Row Count.
  - Columns: Table ID, Column Index, Name, Type, PK, Nullable.
  - Indexes: Index ID, Table ID, Name, Key Column Index, B+ Tree Root Page.

### 2. DDL Lifecycle & Index Backfill
- **`CREATE TABLE`**: Automatically provisions an 8KB slotted heap page and registers metadata.
- **`CREATE INDEX`**: Provisions a B-link tree root page and provides automatic index bootstrapping by scanning existing table records.
- **`DROP TABLE`**: Cascades deletion of all column descriptors and dependent indexes.
- **`ALTER TABLE ADD COLUMN`**: Enables non-destructive schema evolution with backward-compatible tuple decoding.

## Consequences
- Eliminates hardcoded schemas; SQL parser and optimizer resolve table/column names directly via the catalog.
- Enables multi-table databases with multiple secondary B+ Tree indexes per table.
