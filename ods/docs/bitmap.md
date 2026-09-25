# Storage bitmap (BITMAP.SYS, FID 2,2)

A contiguous file: the storage control block (SCB) in VBN 1, then one bit
per cluster from VBN 2 on, **set when the cluster is free**. Bit j covers
LBNs j·v to j·v + v - 1; bits past the last cluster are clear. The file has
`clusters / 4096` blocks of bits, rounded up; the end of file is the block
after the last one.

## Storage control block (SCB$)

Defined in `crates/ods-core/src/layout/scb.rs`.

| Offset | Size | Field | Notes |
| --- | --- | --- | --- |
| 0 | 2 | `STRUCLEV` | 0x0201 |
| 2 | 2 | `CLUSTER` | same as the home block |
| 4 | 4 | `VOLSIZE` | volume size in blocks |
| 8 | 4 | `BLKSIZE` | physical blocks per logical block (1) |
| 12 | 4 | `SECTORS` | geometry: sectors per track |
| 16 | 4 | `TRACKS` | tracks per cylinder |
| 20 | 4 | `CYLINDER` | cylinders |
| 24 | 4 | `STATUS` | volume status flags |
| 28 | 4 | `STATUS2` | copy while mounted for write |
| 32 | 2 | `WRITECNT` | systems with the volume mounted for write |
| 34 | 12 | `VOLOCKNAME` | lock name: the label |
| 46 | 8 | `MOUNTTIME` | last write mount |
| 54 | 2 | `BACKREV` | BACKUP/IMAGE count |
| 56 | 8 | `GENERNUM` | shadow set generation |
| 64 | 446 | | reserved |
| 510 | 2 | `CHECKSUM` | sum of words 0-254; zero on VMS V1 |

`ods` never writes the SCB after INITIALIZE: VMS keeps the free space count
in memory and derives it from the bitmap at mount.

## Allocation

Clusters are marked used in the bitmap before any header points at them,
and freed only after no header does. Three policies:

- **Any**: first fit, from the end of the file being extended when that
  space is free, so extents merge.
- **Best try** (`FCH$V_CONTIGB`): the largest free runs first.
- **Contiguous** (`FCH$V_CONTIG`): one run, adjacent to the file when
  extending a contiguous file, or fail.
