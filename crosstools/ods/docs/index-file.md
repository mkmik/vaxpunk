# Index file (INDEXF.SYS, FID 1,1)

The root of the volume: it maps the boot block, the home blocks, a backup of
its own header, the index file bitmap and every file header.

| VBN | Contents |
| --- | --- |
| 1 | boot block (LBN 0) |
| 2 | primary home block (LBN 1) |
| 3 to 2v | copies of the home block filling the first two clusters |
| 2v+1 to 3v | the cluster holding the backup home block, filled with copies |
| 3v+1 | backup copy of the index file header; the rest of the cluster unused |
| 4v+1 to 4v+m | index file bitmap, m = `IBMAPSIZE` blocks |
| 4v+m+n | header of file number n |

(v is the cluster size.) The bitmap and the first 16 headers are one
contiguous extent, so the index file's own header (file 1) is at
`IBMAPLBN + IBMAPSIZE` and can be read before anything is mapped. The rest
of the map, and headers above 16, come from that header (and its extension
headers, found through the part of the map read so far).

## Index file bitmap

Bit n-1 (bits packed low to high in increasing bytes) is set when file
number n is in use. It is advisory: a slot holding a valid header is never
reused, whatever its bit says.

## End of file

`EFBLK` of INDEXF.SYS is at or past the last header ever used. Below it,
every slot is a header: valid, deleted, or an empty template. Past it, VMS
considers slots never used and takes them for new files without looking.
So:

- `ods` moves the end of file before writing a header past it, and writes
  empty templates into slots before the end of file covers them.
- The verifier reads every slot the index file maps and reports a valid
  header past the end of file as an error (the DUNGEON fixture has five,
  added by some tool that did not move it); `verify --repair-bitmap` moves
  the end of file past them.

## Allocating a header

The first file number above `RESFILES` whose bit is clear and whose slot
holds no valid header. Its sequence number is one above the one in the slot
(skipping zero), or 1 for a slot past the end of file that never held a
deleted header. The end of file moves first, then the bit is set, then the
caller writes the header, then the directory entry: a crash anywhere in
between leaves at most a used bit or an unreachable file.

When the index file has no slot left, it grows by at least half its header
space (so the map stays short). The index file never gets an extension
header from `ods`; a volume whose index file map is full reports the index
file full.

## Backup header

Every write of the index file's header also writes its backup copy at
`ALTIDXLBN`. VMS V1 did not; the verifier reports a difference as a
warning.
