# Home block (HM2$)

Identifies the volume and says where the index file is. The primary copy is
VBN 2 of INDEXF.SYS, normally LBN 1. Defined in
`crates/ods-core/src/layout/home.rs`.

| Offset | Size | Field | Notes |
| --- | --- | --- | --- |
| 0 | 4 | `HOMELBN` | LBN of this copy |
| 4 | 4 | `ALHOMELBN` | LBN of the backup home block |
| 8 | 4 | `ALTIDXLBN` | LBN of the backup index file header |
| 12 | 2 | `STRUCLEV` | high byte: structure level (2 or 5); low byte: version (1) |
| 14 | 2 | `CLUSTER` | blocks per cluster |
| 16 | 2 | `HOMEVBN` | VBN of this copy in INDEXF.SYS |
| 18 | 2 | `ALHOMEVBN` | VBN of the backup home block: 2v+1 plus its offset in its cluster |
| 20 | 2 | `ALTIDXVBN` | 3v+1 |
| 22 | 2 | `IBMAPVBN` | VBN of the index file bitmap: 4v+1 |
| 24 | 4 | `IBMAPLBN` | LBN of the index file bitmap |
| 28 | 4 | `MAXFILES` | maximum number of files, below 2^24 |
| 32 | 2 | `IBMAPSIZE` | index file bitmap blocks: MAXFILES / 4096, rounded up |
| 34 | 2 | `RESFILES` | reserved files (9 up to VMS 6.0, 10 with SECURITY.SYS) |
| 36 | 2 | `DEVTYPE` | 0 |
| 38 | 2 | `RVN` | relative volume number, 0 off volume sets |
| 40 | 2 | `SETCOUNT` | volumes in the set |
| 42 | 2 | `VOLCHAR` | bits: 0 read check, 1 write check, 2 erase on delete, 3 no highwater marking, 4 classification checks, 5 access times, 6 hard links |
| 44 | 4 | `VOLOWNER` | owner UIC: member in the low word, group in the high one |
| 48 | 4 | | reserved |
| 52 | 2 | `PROTECT` | volume protection |
| 54 | 2 | `FILEPROT` | default file protection |
| 56 | 2 | `RECPROT` | "reserved" in the 1990 book; VMS V1 wrote 0xE000, VMS 7.1 0xFE00 |
| 58 | 2 | `CHECKSUM1` | sum of words 0-28 |
| 60 | 8 | `CREDATE` | creation time |
| 68 | 1 | `WINDOW` | default window size (7) |
| 69 | 1 | `LRU_LIM` | directory cache limit (3) |
| 70 | 2 | `EXTEND` | default extend (5) |
| 72 | 8 | `RETAINMIN` | minimum retention, delta time |
| 80 | 8 | `RETAINMAX` | maximum retention |
| 88 | 8 | `REVDATE` | revision time |
| 96 | 20 | `MIN_CLASS` | security classification |
| 116 | 20 | `MAX_CLASS` | |
| 136 | 320 | | reserved (later VMS versions use some) |
| 456 | 4 | `SERIALNUM` | media serial number |
| 460 | 12 | `STRUCNAME` | volume set name, spaces if none |
| 472 | 12 | `VOLNAME` | label, space padded |
| 484 | 12 | `OWNERNAME` | space padded |
| 496 | 12 | `FORMAT` | `DECFILE11B  ` |
| 508 | 2 | | reserved |
| 510 | 2 | `CHECKSUM2` | sum of words 0-254 |

A block is a valid home block when both checksums match, the structure level
is 2 or 5 with a version of at least 1, `HOMELBN`, `ALHOMELBN`,
`ALTIDXLBN`, `CLUSTER`, `HOMEVBN`, `IBMAPVBN`, `IBMAPLBN` and `IBMAPSIZE`
are nonzero, `RESFILES` is at least 5 and `MAXFILES` is above it and fits
the index file bitmap. `mount` also wants `HOMELBN` to be where the copy
was found.

## Copies

All copies are identical except `HOMELBN`, `HOMEVBN` and the checksums. The
index file's first two clusters hold the boot block (VBN 1, LBN 0), the
primary home block (VBN 2, LBN 1) and copies filling the rest; its third
cluster holds nothing but copies, one of which is the backup home block.

## Search sequence

When LBN 1 is bad, VMS looks at LBN 1 + n·delta, where delta comes from the
disk geometry (sectors s, tracks t, cylinders c):

| Geometry | Delta |
| --- | --- |
| s×1×1, 1×t×1, 1×1×c | 1 |
| s×t×1, s×1×c | s + 1 |
| 1×t×c | t + 1 |
| s×t×c | (t + 1)·s + 1 |

The RK07 fixtures (22×3×815) have their backup home block at 1 + 89 = 90,
as the table says. `ods` does not know the geometry of an image, so after
LBN 1 it tries every block of the first 65,536.
