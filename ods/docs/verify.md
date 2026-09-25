# Verifier

`ods verify` (`Volume::verify` in the core) walks the whole volume, much
like ANALYZE/DISK_STRUCTURE, and sorts what it finds into three classes:

- **Error**: can lose data or confuse VMS. Blocks in use but free in the
  bitmap; blocks in two files (or twice in one); a directory entry naming a
  missing file or a stale file ID; a bad header checksum below the index
  file's end of file; a valid header past it; a directory out of order or
  with versions out of order; an unparsable directory block; a broken
  extension header chain; a reserved file missing.
- **Leak**: space nobody can use. Clusters allocated in the bitmap that no
  file maps; file numbers marked in use without a header; files no
  directory lists; extension headers of no file; an allocation size below
  what the headers map.
- **Warning**: harmless disagreement. A back link that is not a directory
  listing the file; an allocation size above what is mapped; an end of file
  past the allocation; a file marked contiguous that is not; extents not in
  whole clusters; a stale backup index header or alternate home block; a
  valid header the index file bitmap forgot.

What it reads: the alternate home block and backup index header; every
header slot the index file maps (with each file's chain of extension
headers); the storage and index file bitmaps; every directory reachable from
the MFD.

`--repair-bitmap` rewrites both bitmaps from what the headers use, and moves
the index file's end of file past any valid header beyond it. Lost files
stay lost (VMS's `/REPAIR` would enter them in `[SYSLOST]`).

`ods verify` exits with 0 when there are no errors, 2 when there are.

## Crash behaviour

Files-11 has no journal. `ods` orders its writes as VMS does: space is
allocated in the bitmaps before anything points at it, a header is written
before the directory entry naming it, an entry is removed before its file
is deleted. Directory and map changes are shaped so that any prefix of their
writes is consistent (see [directory.md](directory.md) and
[file-header.md](file-header.md)). The tests in
`crates/ods-core/tests/model.rs` check this: they replay random operation
sequences cutting the power after every possible write, and require every
state left behind to mount and verify without errors. Leaks are allowed.
