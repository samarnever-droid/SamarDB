# 03 State Graph — SamarDB

## On-Disk & Memory State Shapes

```mermaid
graph TD
    subgraph Buffer Pool Frame
        BP[BufferFrame] -->|Contains| PageBuf[8KB Page Buffer]
        BP -->|Tracks| Pin[pin_count]
        BP -->|Tracks| Dirty[is_dirty]
    end

    subgraph Page Layout
        PageBuf --> Hdr[32-byte PageHeader]
        PageBuf --> Items[Item Pointers Array lower->]
        PageBuf --> FreeSpace[Free Space lower..upper]
        PageBuf --> Tuples[Tuple Data <-upper]
    end

    subgraph WAL Record
        WAL[40-byte WalRecordHeader] --> LSN[Monotonic LSN]
        WAL --> CRC[CRC32C]
        WAL --> Payload[Txn Mutation Data]
    end

    subgraph Storage
        Disk[SimDisk / POSIX File]
    end

    Dirty -->|Eviction / Flush| Disk
    WAL -->|Flush Before Page (I2)| Disk
```
