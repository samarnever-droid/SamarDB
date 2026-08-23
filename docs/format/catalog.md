# Format Spec: System Catalog & DDL Metadata (`samardb-catalog`)
# Version: 1

## Overview

SamarDB Phase 13 specifies the system catalog layout and dynamic DDL (Data Definition Language)
metadata structures for tracking tables, columns, indexes, and schema evolution.

---

## 1. System Catalog Tables

The system catalog uses three foundational metadata tables:

### 1.1 `samar_tables` (Table ID: 1)
| Column | Field Name       | Type     | Description                                 |
|--------|------------------|----------|---------------------------------------------|
| `0`    | `table_id`       | `INT`    | Unique 64-bit integer identifier for table  |
| `1`    | `table_name`     | `VARCHAR`| Table name string (case-insensitive)        |
| `2`    | `heap_root_page` | `INT`    | Initial 8KB heap page ID allocated for data |
| `3`    | `row_count`      | `INT`    | Approximate tuple count for optimizer stats |

### 1.2 `samar_columns` (Table ID: 2)
| Column | Field Name       | Type     | Description                                 |
|--------|------------------|----------|---------------------------------------------|
| `0`    | `table_id`       | `INT`    | Foreign key referencing `samar_tables`      |
| `1`    | `col_idx`        | `INT`    | 0-indexed column position in tuple layout   |
| `2`    | `col_name`       | `VARCHAR`| Column name string                          |
| `3`    | `data_type`      | `INT`    | Type code (`1=INT`, `2=VARCHAR`, `3=BOOL`)  |
| `4`    | `is_pk`          | `BOOL`   | Whether column is the table Primary Key     |
| `5`    | `is_nullable`    | `BOOL`   | Whether column allows NULL values           |

### 1.3 `samar_indexes` (Table ID: 3)
| Column | Field Name        | Type     | Description                                |
|--------|-------------------|----------|--------------------------------------------|
| `0`    | `index_id`        | `INT`    | Unique 64-bit integer index identifier     |
| `1`    | `table_id`        | `INT`    | Table ID on which index is defined         |
| `2`    | `index_name`      | `VARCHAR`| Index name string                          |
| `3`    | `key_col_idx`     | `INT`    | Indexed column index in table schema       |
| `4`    | `btree_root_page` | `INT`    | Root page ID of the Lehman-Yao B-link tree |

---

## 2. Dynamic DDL Operations

1. **`CREATE TABLE <name> (<columns>)`**:
   - Generates new `table_id`.
   - Allocates heap root page.
   - Registers table in `samar_tables` and columns in `samar_columns`.
2. **`DROP TABLE <name>`**:
   - Deregisters table from `samar_tables`.
   - Deregisters all columns from `samar_columns`.
   - Drops any associated secondary indexes in `samar_indexes`.
3. **`CREATE INDEX <idx_name> ON <table_name> (<col_name>)`**:
   - Allocates B+ Tree root page.
   - Registers index in `samar_indexes`.
   - Scans existing heap pages to populate index entries (index backfill).
4. **`ALTER TABLE <name> ADD COLUMN <col_def>`**:
   - Increments column count in schema descriptor.
   - Appends new entry to `samar_columns` with default nullability.
