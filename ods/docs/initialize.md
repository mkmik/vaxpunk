# INITIALIZE

`ods_core::initialize` builds an empty volume on a blank device. It copies
what VMS 7.1's INITIALIZE wrote on the `vms-7.1-init.dsk` fixture, where
that is not about a physical disk.

## Parameters and defaults

| Parameter | Default |
| --- | --- |
| cluster | 1 up to 50,000 blocks, else 3; more if needed to keep the storage bitmap within 255 blocks (the limit older VMS versions have) |
| maximum files | volume size / ((cluster + 1) × 2), as VMS computes it (the fixture: 2,940,951 / 8 = 367,618) |
| preallocated headers | 16, rounded up to fill the last cluster |
| owner | [1,4] |
| volume protection | none denied |
| default file protection | S:RWED,O:RWED,G:RE,W: (0xFA00) |

## Layout

- Clusters 0 and 1: boot block (zeros; VMS writes a "not a system disk"
  program), primary home block at LBN 1, home block copies.
- The backup home block cluster at LBN 1 + delta. An image has no geometry,
  so the SCB records a made-up one, `sectors × 1 × cylinders` with at least
  32 sectors and at least twice the cluster size: the search delta is then
  sectors + 1, which lands past the first two clusters.
- From the middle of the volume, as VMS's default `/INDEX=MIDDLE` does:
  000000.DIR (one cluster), BITMAP.SYS (SCB and bitmap), the index file
  bitmap and headers, and the backup index file header cluster.
- A partial last cluster goes to BADBLK.SYS, as VMS does.

The primary home block is written last, so an interrupted INITIALIZE leaves
no volume rather than a broken one.

## Reserved files

| FID | Name | Record format | Characteristics | Blocks |
| --- | --- | --- | --- | --- |
| 1,1 | INDEXF.SYS | FIX 512 | | as above |
| 2,2 | BITMAP.SYS | FIX 512 | CONTIG | SCB + bitmap |
| 3,3 | BADBLK.SYS | FIX 512 | | partial last cluster, if any |
| 4,4 | 000000.DIR | VAR 512, BLK | CONTIG, DIRECTORY | 1 cluster |
| 5,5 | CORIMG.SYS | FIX 512 | | 0 |
| 6,6 | VOLSET.SYS | FIX 64 | | 0 |
| 7,7 | CONTIN.SYS | FIX 512 | | 0 |
| 8,8 | BACKUP.SYS | FIX 64 | | 0 |
| 9,9 | BADLOG.SYS | FIX 16 | | 0 |

All are owned by the volume owner with the default file protection, except
the MFD, which also grants world execute (S:RWED,O:RWED,G:RE,W:E). Each has
an ODS-2 ident area with its name, revision 1, creation and revision dates
the initialization time; the back link is the MFD; the highwater mark is
the allocation plus one (the MFD's: 2). The MFD lists all nine, version
limit 1.

VMS 6.1 and later also create SECURITY.SYS (10,10), a volume security
profile; VMS 5.5 and 6.0 CDs show volumes are complete without it.
