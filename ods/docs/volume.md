# Volumes, blocks and file IDs

A volume is an array of 512-byte **logical blocks**, numbered from 0 (LBN).
Files-11 knows no other block size. Blocks are grouped into **clusters** of
`cluster` blocks (the home block's `HM2$W_CLUSTER`), the unit of allocation:
every extent of a file starts on a cluster boundary and is a whole number of
clusters long. The one exception seen on real disks is BADBLK.SYS, which
INITIALIZE gives the partial last cluster when the volume size is not a
multiple of the cluster size; that cluster can reach past the end of the
device (the RK07 fixtures show it).

A file is an array of **virtual blocks** numbered from 1 (VBN), mapped to
logical blocks by the retrieval pointers in its header(s).

## File ID (FID)

Six bytes, wherever a file is named (headers, directory entries, back links):

| Offset | Size | Field |
| --- | --- | --- |
| 0 | 2 | `FID$W_NUM`, file number, low 16 bits |
| 2 | 2 | `FID$W_SEQ`, sequence number |
| 4 | 1 | `FID$B_RVN`, relative volume number (0 off volume sets) |
| 5 | 1 | `FID$B_NMX`, file number, high 8 bits |

File numbers are 24 bits and start at 1; multiples of 65,536 are never
used. The sequence number goes up by one each time a file number is reused,
so a FID naming a deleted file is recognizably stale. `ods` prints FIDs as
`(num,seq,rvn)`.

## Checksums

The home block, file headers and the SCB end in a 16-bit word that is the
sum, modulo 2^16, of all the words before it (the home block also has a
first checksum over its first 29 words). VMS V1 left the SCB checksum zero;
zero is accepted there.

## Volume sets

Not supported: `mount` refuses volumes whose home block has an RVN above 1
or a set count above 1.

## Containers

Images are raw blocks. Two variations are recognized:

- **simh disks**: Open SIMH appends one block to the image. It starts with
  `simh`, then the simulator name (64 bytes), the drive type (16 bytes, at
  offset 68, e.g. `RK07`), the sector size and sector count as big-endian
  32-bit words at offsets 84 and 88, and a creation date. The volume is the
  first `size * count` bytes.
- **Dual-format CDs** (ISO 9660 and ODS-2): the ISO 9660 structures sit in
  blocks the ODS-2 volume marks as used, so the ODS-2 side is just the raw
  image. Nothing to do.
