# Indexed files, prologue 3

An indexed file keeps its records in primary key order in data buckets,
with an index of buckets above them for each key: the primary key's index
leads to the records, an alternate key's to SIDRs, secondary index data
records, which give the records' addresses. Layouts here are what OpenVMS
Alpha V8.4-2L1's `CONVERT` and RMS write, read from the fixtures in
`fixtures/rms` with `ANALYZE/RMS_FILE/INTERACTIVE`, whose field names they
use. Every integer is little endian; a VBN counts blocks of the file from 1.

The file header says what the file is (`file-header.md`): `FAT$B_RTYPE` is
`0x20` (indexed) plus the record format, `FIX` 1 or `VAR` 2;
`FAT$B_BKTSIZE` is the largest bucket size of its areas (`idxf.idx`, with
areas of 2, 3 and 1 blocks, says 3); `FAT$W_MAXREC` the longest record, 0
for any, and `FAT$W_RSIZE` the record size of a `FIX` file, 0 for `VAR`;
the end of file is the end of the allocation (`EFBLK` = `HIBLK` + 1,
`FFBYTE` 0).

## The prologue

VBN 1 holds key descriptor 0 at offset 0 and the fixed prologue after it;
more key descriptors follow, chained, then the area descriptors, at
`PLG$B_AVBN`, each 64 bytes. Every prologue block ends in a checksum word,
the 16-bit sum of the block's first 255 words. `CONVERT` and `CREATE/FDL`
both pack the alternate key descriptors 102 bytes apart from offset 0 of
VBN 2, five to a block (`idxf.idx` has keys 1 and 2 at offsets 0 and 102,
`SYSUAF.DAT` keys 1, 2 and 3 at 0, 102 and 204), the areas starting in the
block after; a reader follows the chain whatever the packing. Area *n* is
at offset 64 × (*n* mod 8) of block `AVBN` + *n* / 8, eight to a block, the
checksum falling in the eighth's unused tail; no fixture has more than
three areas, so the eight is RMS's arithmetic as we assume it, not
something seen.

### Fixed prologue (`PLG$`, in VBN 1)

| Offset | Size | Field | Notes |
| --- | --- | --- | --- |
| 0x66 | 1 | `AVBN` | VBN of the first area descriptor |
| 0x67 | 1 | `AMAX` | number of areas |
| 0x74 | 2 | `VER_NO` | prologue version: 3 |

The rest of 0x60-0x7F is 0 in an indexed file; a relative file uses it
(`relative.md`).

### Key descriptor (`KEY$`, 102 bytes)

| Offset | Size | Field | Notes |
| --- | --- | --- | --- |
| 0 | 4 | `IDXFL` | VBN of the next key descriptor, 0 after the last |
| 4 | 2 | `NOFF` | its offset in that block |
| 6 | 1 | `IANUM` | area of the index buckets above level 1 |
| 7 | 1 | `LANUM` | area of the level 1 index buckets |
| 8 | 1 | `DANUM` | area of the data buckets |
| 9 | 1 | `ROOTLEV` | level of the root bucket: 1 when one index bucket points at every data bucket |
| 10 | 1 | `IDXBKTSZ` | index bucket size, in blocks |
| 11 | 1 | `DATBKTSZ` | data bucket size |
| 12 | 4 | `ROOTVBN` | the root bucket |
| 16 | 1 | `FLAGS` | below |
| 17 | 1 | `DATATYPE` | below |
| 18 | 1 | `SEGMENTS` | 1-8 |
| 19 | 1 | `NULLCHAR` | the null value, with `NULKEYS` |
| 20 | 1 | `KEYSZ` | the key's size: its segments' sizes added |
| 21 | 1 | `KEYREF` | key of reference: 0 primary, 1-254 alternate |
| 22 | 2 | `MINRECSZ` | shortest record that holds the key: the end of its last byte |
| 24 | 2 | `IDXFILL` | bytes of an index bucket a load fills |
| 26 | 2 | `DATFILL` | bytes of a data bucket a load fills |
| 28 | 16 | `POSITION` | 8 words: each segment's offset in the record |
| 44 | 8 | `SIZE` | 8 bytes: each segment's size |
| 52 | 32 | `KEYNAM` | the key's name, space padded; zeros if it has none |
| 84 | 4 | `LDVBN` | first data bucket (SIDR bucket for an alternate key) |
| 88 | 8 | `TYPE` | 8 bytes: each segment's data type; 0 for string |
| 96 | 6 | | 0 |

`FLAGS`: bit 0 `DUPKEYS`, duplicates allowed; 1 `CHGKEYS`, `$UPDATE` may
change it (alternate keys only); 2 `NULKEYS`, records whose key is all
`NULLCHAR` aren't in its index (alternate only); 3 `IDX_COMPR`, index
compression; 4 `INITIDX`, the index isn't there yet: a file `$CREATE` made
has it, with `ROOTVBN`, `ROOTLEV` and `LDVBN` 0, and the first `$PUT`
makes the key's first data (or SIDR) bucket and a root of level 1 above
it, and clears it; 6 `KEY_COMPR`, key compression in data or SIDR
buckets; 7 `REC_COMPR`, data record compression (primary only).

`DATATYPE`: 0 string, 1 signed word, 2 unsigned word, 3 signed long, 4
unsigned long (FDL `bin4`), 5 packed decimal, 6 signed quad, 7 unsigned
quad (`bin8`), 8 collated; 32 + n is n descending. Strings compare as
unsigned bytes, integers as integers; a segmented key is a string.

### Area descriptor (`AREA$`, 64 bytes)

| Offset | Size | Field | Notes |
| --- | --- | --- | --- |
| 0 | 1 | `FLAGS` | 0 |
| 2 | 1 | `AREAID` | its number |
| 3 | 1 | `ARBKTSZ` | bucket size, in blocks |
| 4 | 2 | `VOLUME` | relative volume, 0 |
| 6 | 1 | `ALN` | placement, 0 for none |
| 7 | 1 | `AOP` | allocation options |
| 8 | 4 | `AVAIL` | first reclaimed bucket, 0 for none (`CONVERT/RECLAIM`) |
| 12 | 4 | `CVBN` | start of the area's current extent |
| 16 | 4 | `CNBLK` | its blocks |
| 20 | 4 | `USED` | blocks of it used |
| 24 | 4 | `NXTVBN` | next VBN to use: `CVBN` + `USED` |
| 28 | 4 | `NXT` | start of the next extent, 0 if none |
| 32 | 4 | `NXBLK` | its blocks |
| 36 | 2 | `DEQ` | default extension, blocks |
| 40 | 4 | `LOC` | placement location |
| 44 | 6 | `RFI` | related file |
| 50 | 4 | `TOTAL_ALLOC` | blocks the area has |

A new bucket comes from `NXTVBN`; when the extent runs out, the file is
extended by at least `DEQ` blocks (rounded up to whole buckets) and the
new blocks become the current extent.

## Buckets

A bucket is `ARBKTSZ` blocks, at the VBN it is named by. Its first and
last bytes are its check character, which each write increments: a
reader that finds them different read half a bucket.

### Bucket header (`BKT$`, 14 bytes)

| Offset | Size | Field | Notes |
| --- | --- | --- | --- |
| 0 | 1 | `CHECKCHAR` | as the bucket's last byte |
| 1 | 1 | `INDEXNO` | key of reference |
| 2 | 2 | `ADRSAMPLE` | low word of its VBN |
| 4 | 2 | `FREESPACE` | offset of its first free byte |
| 6 | 2 | `NXTRECID` | next record ID to give (data buckets); 1 in index and SIDR buckets |
| 8 | 4 | `NXTBKT` | next bucket of the same level, in key order; the last points at the first |
| 12 | 1 | `LEVEL` | 0 for data and SIDR buckets |
| 13 | 1 | `BKTCB` | bit 0 `LASTBKT`, the last of its level; bit 1 `ROOTBKT`; bits 3-4 `PTR_SZ`, index buckets' pointer size less 2 |

Each level is a chain from its first bucket (the key descriptor's `LDVBN`
for level 0) to the one with `LASTBKT`.

### Index buckets

Keys from offset 14 up to `FREESPACE`, ascending; bucket pointers from the
end down: the last byte is the check character, the byte before it 0, the
word before that `VBN FREE SPACE`, the offset of the last free byte below
the pointers. Pointer *i*, `PTR_SZ` + 2 bytes, is at `size - 4 - (i+1) *
(PTR_SZ+2)` and goes with key *i*: the bucket below whose keys are all at
most that key. The last bucket of each level has the highest possible key
in its last entry, all ones (`FF`s), so every key finds a bucket.

Without `IDX_COMPR` each key is `KEYSZ` bytes. With it, each is a length
byte, a front count, and that many bytes (see *Key compression*).

`CONVERT` makes the index key of a bucket the highest key in it; RMS's
`$PUT` keeps whatever it chose at the split.

### Primary data records (`IRC$`)

From offset 14 up to `FREESPACE`, in key order:

| Size | Field | Notes |
| --- | --- | --- |
| 1 | `CONTROL` | bits 0-1 `PTRSZ`, the pointer's size less 2; 2 `DELETED`; 3 `RRV`; 4 `NOPTRSZ` |
| 2 | `ID` | record ID, unique in the bucket, from `NXTRECID` |
| 2 | `RRV_ID` | the record's RFA: its ID |
| 2-5 | `RRV_VBN` | and its bucket (`PTRSZ` + 2 bytes; 4 as written) |
| 2 | size | the bytes that follow; absent for `FIX` records without compression |
| | | the record: as it is, or compressed |

A record's RFA is where it was first put: (`RRV_VBN`, `RRV_ID`). When a
split moves it, it keeps its RFA and leaves an RRV, a record reference
vector, in the bucket it came from, with its old ID; every SIDR pointer and
RFA a program keeps go through it. A record that moves again updates that
RRV; it never leaves a second.

An RRV is `CONTROL` with `RRV` set, its `ID`, and a pointer to where the
record is now: an ID (2) and a VBN (`PTRSZ` + 2). RRVs sit after the
records in their bucket.

`$DELETE` takes the record out of its bucket; a record a recovery unit
deleted keeps `DELETED`, which a reader skips, and so does an RRV whose
record is gone.

### Key compression

With `KEY_COMPR` (data and SIDR buckets) or `IDX_COMPR` (index buckets) a
key is stored as

| Size | Field |
| --- | --- |
| 1 | length of what follows |
| 1 | front count: leading bytes it shares with the key before it in the bucket |
| | the bytes |

The key is the front count's bytes of the previous key, the bytes, and then
the last byte repeated up to `KEYSZ`: trailing repeats are cut, all but
one. The first key in a bucket has a front count of 0. So `K00000` is `02
00 4B 30`, then `K00001` after it `01 05 31`, `K10000` after that `02 01
31 30`, and an index's high key `01 00 FF`.

### Data record compression

With `KEY_COMPR` the primary key is taken out of the record and stored
first, compressed; with `REC_COMPR` what is left of the record follows as
segments:

| Size | Field |
| --- | --- |
| 2 | length of the literal bytes |
| | the bytes |
| 1 | how many more times the last of them repeats |

A run of one byte becomes its first byte and a count; a reader takes any.
In the fixtures RMS leaves what is left of a record whole when it is 8
bytes or fewer (`idxb.idx`'s ` yyyyyyy`), and otherwise compresses runs of
6 or more and a run of 5 at the end (`comp.idx`'s 5 trailing blanks, but
not its `yyyyy` in the middle); our loader does the same. A count is at
most 255, so a longer run goes on in another segment, whose literal is one
more of the byte. The last segment's count is often 0; a record that is
all key has no segments.
So `ccc:K00002:ab    cd          ef` with its key at 4 is the key `01 05 32`
then `0E 00 "ccc::ab    cd " 09 02 00 "ef" 00`. The size word counts the
key and the segments.

### SIDRs

An alternate key's level 0 buckets hold SIDRs, a record per key value, in
key order, after the 14-byte header:

| Size | Field | Notes |
| --- | --- | --- |
| 2 | size | the bytes that follow |
| | key | `KEYSZ` bytes, or compressed with `KEY_COMPR` |
| | pointers | to the records with that key, in the order they were put |

Each pointer is a control byte (`PTRSZ` in bits 0-1, `DELETED` bit 2,
`NOPTRSZ` bit 4, and bit 7 on the first pointer of a SIDR), a record ID
(2) and a VBN (`PTRSZ` + 2): the record's RFA. Without duplicates a SIDR
has one. `CONVERT` writes the pointers in primary key order. A SIDR too big
for a bucket goes on in the next with the same key: our loader's choice,
no fixture has one. RMS writes the smallest pointer that holds the VBN; `CONVERT`
always 4 bytes. `$DELETE`, or an `$UPDATE` that changes the key, marks the
pointer: the first keeps its bytes with `DELETED` set, another shrinks to
its control byte alone, `0x14`, `DELETED` and `NOPTRSZ`.

## Loading

`CONVERT` writes the records in primary key order and starts a new data
bucket unless the current one's `FREESPACE` is below the data fill
quantity and the record, sized as it would be first in a bucket (its key
compressed against nothing), ends before the bucket's last two bytes. The
same rule fills SIDR buckets, a SIDR at a time, and reproduces every data
and SIDR bucket of the fixtures. Index buckets fill otherwise: `idxb.idx`
has 40 entries in each level 1 bucket at an index fill of 256 bytes,
which no rule over its byte counts we tried explains; our loader fills an
index bucket while its keys and pointers are below the fill quantity, at
least two entries a bucket. The index key of each bucket is the highest
key in it, and the last of each level is all ones; for a signed integer or
descending key we write the highest of its order instead (`7F` on top of
`FF`s, or zeros for a descending one), unchecked against RMS.

`CONVERT` takes blocks for buckets as it goes, a cluster at a time, so a
level 1 index bucket sits after the first data bucket (`idx1.idx`: data
at VBN 4, root at 5, data again from 6), and an area's extents interleave
with others'. Ours gives each area one extent after the prologue, its
buckets in the order they were made, data before index.

`ANALYZE/RMS_FILE/FDL` gives fill quantities as percentages; `CONVERT`
took `DATA_FILL 256` as bytes (`idxb.idx`'s 512-byte buckets have a fill of
256, reported as 50). Where an FDL file doesn't say, keys and data records
get compression or not by the key: the string keys of 8, 9 and 32 bytes in
the fixtures got it, `idx1.idx`'s 5-byte key 1 and the integer keys didn't;
our FDL reader draws the line at 6 bytes. `LEVEL1_INDEX_AREA` is
`INDEX_AREA` unless given.

## What a writer must keep

- Records in order of their key in each bucket, buckets in order along the
  chain, every key in a bucket at most the index key that leads to it.
- `FREESPACE` and the `VBN FREE SPACE` word right, and both check
  characters equal.
- Record IDs unique in a bucket and below `NXTRECID`; an RFA once given
  stays the record's until it is deleted.
- Every record in each alternate key's SIDRs once, unless `NULKEYS` and
  its key is null, or the record is too short for the key
  (`MINRECSZ`).
- `USED` and `NXTVBN` past every bucket of the area.
