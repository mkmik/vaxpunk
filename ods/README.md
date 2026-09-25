# ods: Files-11 in Rust

A from-scratch implementation of the VMS file system, Files-11 On-Disk
Structure levels 2 and 5, to be vaxpunk's file system later and a reference
to test other implementations against. It reads and writes disk images on
macOS and Linux today.

```
  ods (CLI)       ods-fuse (mount)
        \            /
         ods-image          image files: raw, simh; paths, records, text
             |
          ods-core          all Files-11 logic; no_std, over a BlockDevice
```

| Crate | What |
| --- | --- |
| `crates/ods-core` | Structure layouts, mount, directories, allocation, INITIALIZE, verifier. `#![no_std]` with `alloc`, no dependencies, no `unsafe`; it builds for `aarch64-unknown-none` |
| `crates/ods-image` | The core over image files, with advisory locking; VMS and Unix path forms; byte streams; VAR, VFC, FIX and stream records; text conversion; tree export and import with an attribute manifest; errors printed as VMS status codes |
| `crates/ods-cli` | `ods`, a thin command layer over ods-image |
| `crates/ods-fuse` | `ods-fuse`, a read-only FUSE mount over ods-image, with the mapping proposed in [docs/fuse.md](docs/fuse.md) |

The CLI and the FUSE daemon never touch the core directly; CI checks that.

## Using it

```sh
cargo build --release
ods=target/release/ods

$ods init disk.img --size 100M --label WORK              # --ods5 for ODS-5
$ods mkdir disk.img '[SRC]'
$ods copy-in disk.img README.md '[SRC]README.MD' --mode lines-to-records
$ods copy-in disk.img photo.jpg /src/photo.jpg           # Unix form works too
$ods dir disk.img '[...]' --full --versions
$ods type disk.img '[SRC]README.MD'
$ods copy-out disk.img '[SRC]README.MD' out.md           # text: records-to-lines
$ods rename disk.img '[SRC]PHOTO.JPG;1' '[000000]PIC.JPG'
$ods delete disk.img '[SRC]README.MD;*'                  # a version is required
$ods purge disk.img '[...]*.*' --keep 2
$ods set-attr disk.img '[000000]PIC.JPG' --protection S:RWED,O:RWED,G:R,W:
$ods dump disk.img '[000000]PIC.JPG'                     # or --fid 12,1, or --lbn 1
$ods verify disk.img                                     # exit status 2 on errors
$ods export disk.img '[SRC]' ./src-tree                  # with ods-manifest.json
$ods import disk.img ./src-tree '[COPY]'
```

`info`, `dir`, `dump` and `verify` take `--json`. Wildcards: `*`, `%`, `?`
and `[...]`.

```sh
ods-fuse disk.img /mnt/vms        # --versions also lists older versions as name;N
ls /mnt/vms/SRC; grep -r TODO /mnt/vms; getfattr -d -m vms /mnt/vms/SRC/README.MD
umount /mnt/vms                   # or Ctrl-C
```

Text files read as lines, everything else as bytes; VMS attributes are
`vms.*` extended attributes. On Linux this needs fuse3; on macOS, macFUSE
with its kernel extension allowed (on this project's Mac it is not, so the
mount has been tested in a Linux container).

## Tests

```sh
fixtures/fetch.sh          # real VMS disks: a few MB; --all adds three CDs (1 GB)
cargo test --release
```

- **Fixtures**: VMS V1.0 (1978) to OpenVMS Alpha 8.4-2L1 (2016), plus a
  volume VMS 7.1 INITIALIZEd. Every home block, header and directory block
  must serialize back to its bytes, the verifier must find only the known
  defects, and each must list exactly as `fixtures/expected/` says (content
  checksums included). BACKUP save sets on the CDs are read through the file
  system and must pass their own sequence and XOR checks.
- **Model**: random creates, writes, extends, truncates, renames, deletes,
  purges, version limits, big directories and names with hundreds of
  versions, checked against an in-memory model with the verifier after each
  step (`crates/ods-core/tests/model.rs`; failing seeds go in `SEEDS`).
- **Power loss**: the same sequences cut after every possible write; every
  state must mount and verify with at most leaked space.
- **Fuzzing**: `fuzz/` (nightly and cargo-fuzz): `cargo fuzz run mount`
  mounts and walks arbitrary images, `cargo fuzz run structures` parses a
  block as every structure and a file specification.

## Status

Against the PRD's work order:

| Step | State |
| --- | --- |
| 1. Spec notes, fixtures | done: `docs/`, `fixtures/fetch.sh` |
| 2-5. Home block, headers, directories, reading | done, checked on all fixtures |
| 6. Verifier | done: clean on every fixture except DUNGEON, which has a real defect |
| 7. INITIALIZE | done, modelled on VMS 7.1 |
| 8. Writing | done, with model and power-loss tests |
| 9. Check on real VMS | done: OpenVMS Alpha 8.4 finds nothing wrong with `ods`'s ODS-2 and ODS-5 volumes and BACKUP copies every file off them exactly (`vms/check.py`, [docs/vms-check.md](docs/vms-check.md)) |
| 10. FUSE read-only | done with the mapping [docs/fuse.md](docs/fuse.md) proposes, for the design session to confirm; `ls`, `cat`, `grep` work on the fixtures |
| 11. FUSE read-write | not started: waits for the mapping to be settled |

Known limits: no volume sets, sparse files, UCS-2 names on write, hard
links, or ACL editing (ACLs are kept as they are). Allocation is first fit
with no caching: correct first, fast later.
