# 04 Dependencies & Contracts — SamarDB

## Module Contract Graph
```
bytes.lpp
 ├──> io.lpp (SimDisk, Positional I/O)
 ├──> pager.lpp (32-byte header, CRC32C, LRU eviction)
 │     └──> io.lpp
 ├──> wal.lpp (40-byte header, monotonic LSN, CRC32C)
 ├──> heap.lpp (Slotted page, item pointers, defrag)
 │     └──> pager.lpp
 ├──> recovery.lpp (ARIES REDO scanner)
 │     ├──> pager.lpp
 │     ├──> wal.lpp
 │     └──> heap.lpp
 ├──> tx.lpp (Transaction manager, CLOG bitwise table)
 │     └──> wal.lpp
 ├──> mvcc.lpp (24-byte MVCC header, visibility rules)
 │     └──> tx.lpp
 ├──> vacuum.lpp (MVCC CRUD operations, dead tuple reclamation)
 │     ├──> heap.lpp
 │     ├──> mvcc.lpp
 │     └──> tx.lpp
 ├──> btree.lpp (Lehman-Yao B-link tree, binary search, splits, range scans)
 │     └──> pager.lpp
 ├──> schema.lpp (Record codecs, column descriptors, table schemas)
 ├──> parser.lpp (SQL Lexer, Recursive descent parser, ASTs)
 └──> exec.lpp (Catalog, Volcano operators, DML executors)
       ├──> schema.lpp
       ├──> parser.lpp
       ├──> btree.lpp
       ├──> vacuum.lpp
       ├──> mvcc.lpp
       ├──> heap.lpp
       ├──> tx.lpp
       ├──> wal.lpp
       └──> pager.lpp
```
