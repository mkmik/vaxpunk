# Checking against real VMS

`vms/check.py` has OpenVMS itself judge what `ods` writes (the PRD's work
order step 9). It runs OpenVMS Alpha V8.4-2L1 from its install CD in the
AXPbox Alpha emulator, using the CD's "Execute DCL commands" option, which
needs no licence, and needs about two minutes:

```sh
fixtures/fetch.sh --all      # the OpenVMS CD is one of the fixtures
vms/check.py                 # builds AXPbox and fetches its firmware the first time
```

1. `ods` builds an ODS-2 and an ODS-5 volume and puts under `[T]`
   everything that is hard to get right: nested directories and one of 300
   files (many blocks), 70 versions of one name then purged and one deleted
   (records split over blocks), a file grown across hundreds of one-cluster
   holes (extension headers), renamed and moved files and directories,
   attribute changes, and on ODS-5 names with spaces, several dots,
   accents, mixed case and 120 characters.
2. VMS mounts both read-only and runs `ANALYZE/DISK_STRUCTURE` on them.
3. VMS initializes a volume of its own and `BACKUP`s both `[T]` trees onto
   it: reading every file through its own file system.
4. `ods` verifies VMS's volume and compares every file VMS copied with the
   original: bytes, versions, record attributes, owner, protection, dates
   (not of directories: BACKUP makes those itself).

The check passes when VMS says nothing but that there is no disk quota
file, and every file comes through identical.

## Pieces

| File | What |
| --- | --- |
| `vms/setup.sh` | clones and builds AXPbox (pinned commit, GPL-2.0, not committed), fetches the ES40 SRM firmware (SHA-256 pinned) |
| `vms/es40.cfg` | the emulated AlphaServer ES40: CD and three scratch disks on SCSI |
| `vms/run-vms.py` | boots VMS, answers the date prompt, picks DCL, types a command file, shuts down |
| `vms/check.dcl` | what VMS does in the check |
| `vms/samples.dcl` | how VMS made the `vms-ods5.img` and `vms-ods2.img` fixtures |

## What VMS has taught

Running this found things no test in this repository could, because they
are about what VMS expects rather than what the structure allows:

- **Reserved files on ODS-5 volumes** keep structure level 2 headers with
  ODS-2 ident areas; `ods` gave them level 5 ones and VMS rejected the
  volume.
- **Name types**: VMS gives a name the ODS-2 type whenever ODS-2 would
  accept it uppercased, whatever its case (`lower.txt`); `ods` had called
  those ISO Latin-1.
- **The index file's highwater mark**: VMS reads blocks past a file's
  highwater mark as zeros, and that includes the file headers in
  INDEXF.SYS. `ods` moved the index file's end of file when it took new
  header slots but not the highwater mark, so VMS saw every header past the
  preallocated ones as empty: idle headers marked busy, directory entries
  naming no file, files BACKUP could not open.
- **LRL and MRS**: VMS shows `RSIZE` as the longest record and `MAXREC` as
  the maximum record size; `ods` had them the other way round.
- And one found on the way to VMS: creating a file with its whole size
  allocated at once, in more pieces than a header holds, overflowed the
  header (now it grows extension headers like any extension).
