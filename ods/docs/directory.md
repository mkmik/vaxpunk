# Directories

A directory is a contiguous file with `FCH$V_DIRECTORY` set, VAR records
that do not span blocks (`BLK`), record size 512. Its used blocks are
`EFBLK - 1`; each holds directory records packed from offset 0 and ends
with a word of -1 (0xFFFF), always present, so records use at most 510
bytes. An empty directory is one block holding only the -1 word (`EFBLK`
2); the MFD after INITIALIZE is one block of records.

Defined in `crates/ods-core/src/layout/dir.rs`.

## Record (DIR$)

| Offset | Size | Field | Notes |
| --- | --- | --- | --- |
| 0 | 2 | `SIZE` | bytes after this word; always even |
| 2 | 2 | `VERLIMIT` | the name's version limit; 32767 means none |
| 4 | 1 | `FLAGS` | bits 0-2 type, 0 = list of versions and FIDs (the only one used); on ODS-5 bits 3-5 name type |
| 5 | 1 | `NAMECOUNT` | name length |
| 6 | n | `NAME` | "NAME.TYPE", the dot always there; padded to even length with one byte |
| | 8 each | entries | version (2 bytes) and FID (6), highest version first |

A name with more versions than fit one record (62 for the shortest name)
continues in further records with the same name, versions still going
down, in the same or the following blocks.

## Order

Records are sorted by the bytes of the whole name, "NAME.TYPE", as a
string: so `SYS$SCS.EXE` comes before `SYS.EXE` (`$` < `.`). Across all
fixtures, 3,300 pairs of neighbouring names follow this and none follows
"name, then type". ODS-5 names compare case-blind (ISO Latin-1 uppercase).

## Version limits

A new name gets the directory's default (`VERSIONS` in the directory's
record attributes; 0 there means none, stored as 32767 in the record).
When a new version takes a name past its limit, the lowest versions are
deleted. Reserved files in the MFD have limit 1.

## Updating

Directories change in one of three ways, chosen so that an interrupted
update loses at most an entry (its file becomes an orphan, leaked space)
and never duplicates one or reorders records:

1. **In place**: the change keeps the name's records in one block and the
   block still fits: one block write.
2. **Append**: a new name sorting after every other, when the last block is
   full: its record goes alone into the block after the end of file
   (allocated already, or the free cluster right after the directory), then
   the end of file moves over it. The header write is what makes it appear.
3. **Rewrite**: anything else (a block overflowing in the middle, a name's
   records spreading over blocks, a middle block emptying) writes the whole
   directory, three quarters full per block, to newly allocated contiguous
   space, switches the header's map to it in one write, then frees the old
   space.

When the last block empties, the end of file just moves back.

Real VMS splits blocks in place, shifting the following ones: fast, but a
crash half way leaves duplicated or missing blocks. The power-loss tests
(`crates/ods-core/tests/model.rs`) cut every one of `ods`'s write sequences
after each write and require the result to verify with no errors.
