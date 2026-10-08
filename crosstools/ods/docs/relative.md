# Relative files

A relative file is a prologue block and then buckets of fixed-size cells,
cell *n* holding record number *n*, counted from 1. Layouts are what
OpenVMS Alpha V8.4-2L1's `CONVERT` and RMS write (`fixtures/rms/rel.rel`
and `relf.rel`); see `indexed.md` for the conventions.

The file header: `FAT$B_RTYPE` is `0x10` (relative) plus the record format,
`FIX` 1, `VAR` 2 or `VFC` 3; `FAT$B_BKTSIZE` the bucket size;
`FAT$W_MAXREC` and `FAT$W_RSIZE` both the record size, `FAB$W_MRS`, which a
relative file must have; the end of file is the VBN after the last
bucket in use, as in the prologue.

## The prologue (VBN 1)

| Offset | Size | Field | Notes |
| --- | --- | --- | --- |
| 0x68 | 2 | `PLG$W_DVBN` | first data bucket: 2 |
| 0x6C | 4 | `PLG$L_MRN` | highest record number allowed; `0x7FFFFFFF` for no limit |
| 0x70 | 4 | `PLG$L_EOF` | VBN after the last bucket in use |
| 0x74 | 2 | `PLG$W_VER_NO` | 1 |
| 510 | 2 | | checksum: the sum of the block's first 255 words |

Everything else is 0.

## Buckets and cells

Bucket *b* (from 0) is at VBN `DVBN + b * BKTSIZE`. A bucket has no header:
its cells start at its first byte, as many as fit whole, and the bytes
after the last are unused. A cell is

| Size | Field | Notes |
| --- | --- | --- |
| 1 | control | bit 2 `DLC$V_DELETED`, bit 3 `DLC$V_REC` |
| 2 | size | the record's size, for `VAR` and `VFC` records only |
| `MRS` | | the record, then whatever was there before |

so a `FIX` cell is `MRS + 1` bytes and a `VAR` one `MRS + 3`; record *n* is
in bucket `(n-1) / cells`, cell `(n-1) % cells`. A `VFC` record's size
counts its fixed control part, `FAB$B_FSZ` bytes, which comes first; we
take a `VFC` cell to be `FSZ + MRS + 3` bytes, which no fixture shows.

A cell is empty when its control byte is 0: never written, or past the end
of file. `REC` says it has a record. `$DELETE` sets `DELETED` and leaves the
bytes; a later `$PUT` of that number clears it. `$UPDATE` rewrites the
record in its cell, size and all, and leaves the rest of the cell as it
was.
