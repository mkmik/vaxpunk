# File header (FH2$)

One block in INDEXF.SYS per file, plus one per extension. Four offsets (in
16-bit words) cut the block into areas: header, ident, map, access control
list, reserved; the last word is the checksum. Defined in
`crates/ods-core/src/layout/header.rs`.

## Header area

| Offset | Size | Field | Notes |
| --- | --- | --- | --- |
| 0 | 1 | `IDOFFSET` | start of the ident area, in words. 40 on VMS V4 and later, 38 on VMS V1 (no highwater field) |
| 1 | 1 | `MPOFFSET` | start of the map area (100 with an ODS-2 ident) |
| 2 | 1 | `ACOFFSET` | start of the ACL; 255 when there is none |
| 3 | 1 | `RSOFFSET` | start of the reserved area; 255 when empty |
| 4 | 2 | `SEG_NUM` | 0 for the primary header, n for the nth extension |
| 6 | 2 | `STRUCLEV` | 2.1 (0x0201) or 5.1; decides the ident area format |
| 8 | 6 | `FID` | this header's file ID |
| 14 | 6 | `EXT_FID` | next extension header, zero if none |
| 20 | 32 | `RECATTR` | record attributes, below |
| 52 | 4 | `FILECHAR` | file characteristics, below |
| 56 | 2 | `RECPROT` | "reserved"; VMS 7.1 writes 0xFE00 |
| 58 | 1 | `MAP_INUSE` | map words in use |
| 59 | 1 | `ACC_MODE` | access mode per operation, 2 bits each |
| 60 | 4 | `FILEOWNER` | owner UIC |
| 64 | 2 | `FILEPROT` | protection: 4 bits each for system, owner, group, world; a set bit denies read, write, execute, delete |
| 66 | 6 | `BACKLINK` | directory holding the primary entry; for an extension header, the primary header. VMS V1 headers keep other data here |
| 72 | 1 | `JOURNAL` | journaling flags |
| 73 | 1 | `RU_ACTIVE` | recovery unit facility |
| 74 | 2 | `LINKCOUNT` | ODS-5 hard links; reserved on ODS-2 |
| 76 | 4 | `HIGHWATER` | VBN + 1 of the highest block written; only there when `IDOFFSET` is at least 40 |
| 80 | 8 | | reserved |
| 88 | 20 | `CLASS_PROT` | classification, only when the header area reaches it |
| 510 | 2 | `CHECKSUM` | sum of words 0-254 |

A valid header has a good checksum, a structure level of 2 or 5 (version at
least 1), `38 <= IDOFFSET <= MPOFFSET <= ACOFFSET <= RSOFFSET`, a map not
longer than its area, and a nonzero file number.

Two kinds of free slot are not errors:

- **Deleted**: `FCH$V_MARKDEL` set, file number and RVN zero, checksum
  zero. The sequence number stays, for the next user of the slot.
- **Empty template**: a valid checksum and offsets, file number 0, name
  `.;`. VMS writes these into index file blocks before its end of file
  covers them; the VMS CDs are full of them, and `ods` does the same.

## File characteristics (`FCH$`)

Bit 0 WASCONTIG, 1 NOBACKUP, 2 WRITEBACK, 3 READCHECK, 4 WRITCHECK,
5 CONTIGB (contiguous best try), 6 LOCKED, 7 CONTIG, 11 BADACL, 12 SPOOL,
13 DIRECTORY, 14 BADBLOCK, 15 MARKDEL, 16 NOCHARGE, 17 ERASE, 18 ALM_AIP,
19 SHELVED. The owner may set NOBACKUP to ERASE except CONTIG (which may
only be cleared) and the file system's own bits (DIRECTORY, MARKDEL,
BADACL, SPOOL, BADBLOCK).

## Record attributes (`FAT$`, 32 bytes at 20)

Kept for RMS; the file system only reads the end of file and allocation.
`RecordAttrs` in the core covers every byte, so they round-trip exactly.

| Offset | Size | Field | Notes |
| --- | --- | --- | --- |
| 0 | 1 | `RTYPE` | low nibble record format: 0 UDF, 1 FIX, 2 VAR, 3 VFC, 4 STM, 5 STMLF, 6 STMCR; high nibble organization: 0 sequential, 1 relative, 2 indexed, 3 direct |
| 1 | 1 | `RATTRIB` | bit 0 FTN, 1 CR (implied carriage control), 2 PRN, 3 BLK (records do not span blocks), 4 MSB record counts |
| 2 | 2 | `RSIZE` | record size (FIX) or maximum (VAR, 0 for none) |
| 4 | 4 | `HIBLK` | blocks allocated. **Stored high word first** |
| 8 | 4 | `EFBLK` | end of file block. **High word first** |
| 12 | 2 | `FFBYTE` | first free byte in the end of file block |
| 14 | 1 | `BKTSIZE` | bucket size |
| 15 | 1 | `VFCSIZE` | VFC control area size (0 means 2) |
| 16 | 2 | `MAXREC` | longest record |
| 18 | 2 | `DEFEXT` | default extend |
| 20 | 2 | `GBC` | global buffer count |
| 22 | 8 | | used by later VMS versions; preserved |
| 30 | 2 | `VERSIONS` | a directory's default version limit |

A file holds `(EFBLK - 1) * 512 + FFBYTE` bytes.

## Ident area

**ODS-2 (`FI2$`, 120 bytes)**, when `STRUCLEV` is 2:

| Offset | Size | Field |
| --- | --- | --- |
| 0 | 20 | `FILENAME`: "NAME.TYPE;VERSION", space padded |
| 20 | 2 | `REVISION`: times closed after a write |
| 22 | 8 | `CREDATE` |
| 30 | 8 | `REVDATE` |
| 38 | 8 | `EXPDATE` |
| 46 | 8 | `BAKDATE` |
| 54 | 66 | `FILENAMEXT`: the rest of the name (86 characters in all) |

VMS V1 headers stop after `BAKDATE` (54 bytes): shorter areas read as if
padded, and only the bytes present are ever written.

**ODS-5 (`FI5$`, 120 to 324 bytes)**, when `STRUCLEV` is 5:

| Offset | Size | Field |
| --- | --- | --- |
| 0 | 1 | `CONTROL`: bits 0-1 name type (0 ODS-2, 1 ISO Latin-1, 3 UCS-2), bit 4 fixed length |
| 1 | 1 | `NAMELEN` |
| 2 | 2 | `REVISION` |
| 4 | 8 | `CREDATE` |
| 12 | 8 | `REVDATE` |
| 20 | 8 | `EXPDATE` |
| 28 | 8 | `BAKDATE` |
| 36 | 8 | `ACCDATE`: last access |
| 44 | 8 | `ATTDATE`: last attribute change |
| 52 | 8 | `EX_RECATTR`: extended record attributes |
| 60 | 16 | `LENGTH_HINT` |
| 76 | 44+ | `FILENAME`, `NAMELEN` bytes, continuing up to 248 bytes: the area grows with the name, and `MPOFFSET` with it |

## Map area and retrieval pointers

`MAP_INUSE` words from `MPOFFSET`, a list of pointers in VBN order. The top
two bits of the first word give the format; counts are stored minus one.

| Format | Size | Count | LBN |
| --- | --- | --- | --- |
| 0 | 2 | placement flags, maps nothing | |
| 1 | 4 | bits 0-7 (1-256 blocks) | bits 8-13 of word 0 (high 6 bits) and word 1: 22 bits |
| 2 | 6 | bits 0-13 (1-16,384) | words 1-2, 32 bits |
| 3 | 8 | bits 0-13 of word 0 high, word 1 low (up to 2^30) | words 2-3 |

An LBN of all ones would be a hole in a sparse file; VMS never supported
them and neither does `ods`.

When the map area fills up, an **extension header** (a new file number,
`SEG_NUM` one higher, `BACKLINK` pointing at the primary header, no ident
area) continues it, linked from the previous header's `EXT_FID`.

`ods` only ever changes the end of a file's map: extending appends to the
last header (merging with its last pointer when the new space follows it),
creating and writing a new extension header before linking it; truncating
shortens or unlinks from the end before freeing. So no crash can leave the
same block mapped twice.
