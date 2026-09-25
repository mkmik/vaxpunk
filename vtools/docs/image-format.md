# ARM64 executable image format

A vaxpunk executable image (EXE) is laid out like an OpenVMS Alpha image: a
header of 512-byte blocks, then the contents of each image section, each
starting on a block boundary. This document lists the changes from Alpha. The
code is `vms-obj/src/exe.rs`.

Status: first version (work order step 2). Only what a statically linked
executable needs is produced. Shareable images, fixups and debug symbol tables
keep their Alpha slots, unused.

## Sources

The Alpha image header isn't in the Linker Utility Manual. The field layouts
here come from GNU binutils (`include/vms/eihd.h`, `eisd.h`, `eiha.h`,
`eihi.h`) and match what binutils' linker writes for alpha-*-vms. Binutils is
GPL-3, so this repository takes facts from it, never code.

## Layout

```
block 1       EIHD fixed part, EIHA, EIHI, EISDs ... alias word (bytes 510-511)
block 2..n    more EISDs, if they don't fit in block 1
then          section contents, each starting at a block, zero-padded to one
```

All header bytes that aren't part of a structure are 0xFF.

## Image header (`EIHD$`)

| Offset | Field | vaxpunk value |
| --- | --- | --- |
| 0 | `MAJORID`, `MINORID` | 3, 0 (the Alpha values) |
| 8 | `SIZE` | header bytes used, up to the end of the EISD list |
| 12 | `ISDOFF` | offset of the first EISD |
| 16 | `ACTIVOFF` | offset of the EIHA |
| 20 | `SYMDBGOFF` | 0: no debug symbol table |
| 24 | `IMGIDOFF` | offset of the EIHI |
| 28 | `PATCHOFF` | 0 |
| 32 | `IAFVA` | 0: no fixup section |
| 40 | `SYMVVA` | 0: no symbol vector |
| 48 | `VERSION_ARRAY_OFF` | 0 |
| 52 | `IMGTYPE` | 1, executable |
| 56 | `SUBTYPE` | 0, native |
| 60 | `IMGIOCNT`, `IOCHANCNT` | 0 |
| 68 | `PRIVREQS` | all ones |
| 76 | `HDRBLKCNT` | number of header blocks |
| 80 | `LNKFLAGS` | `LNKNOTFR` (2) if there is no transfer address |
| 84 | `IDENT`, `SYSVER` | 0 |
| 92 | `MATCHCTL` | 0 |
| 96 | `SYMVECT_SIZE` | 0 |
| 100 | `VIRT_MEM_BLOCK_SIZE` | 16: sections are aligned to 2^16 bytes |
| 104 | `EXT_FIXUP_OFF`, `NOOPT_PSECT_OFF` | 0 |
| **112** | **`ARCH`** | **183, ARM64** |
| 510 | `ALIAS` | 0xFFFF |

**Change:** Alpha images have no architecture field, because only Alpha ran
them. vaxpunk puts one at offset 112, which Alpha leaves as fill, with the same
code as the object format (183, ELF's `EM_AARCH64`). A reader rejects any other
value. The EIHA and friends follow the fixed part, 8-byte aligned; readers find
them through the offsets.

## Activation (`EIHA$`)

48 bytes: size, a spare longword, transfer addresses 1 to 4 (8 bytes each), and
`INISHR`. vaxpunk writes only the first transfer address, the image entry point.

**Change (provisional):** on Alpha a transfer address is a procedure
descriptor. Until the calling standard defines ARM64 descriptors, it is the
address of the first instruction.

## Identification (`EIHI$`)

104 bytes: major and minor id (1, 2), link time as a VMS time (100 ns units
since 17-Nov-1858), then counted strings for the image name (40 bytes), image
ident, linker ident and build ident (16 bytes each). Unchanged.

## Image section descriptors (`EISD$`)

36 bytes each, unchanged:

| Offset | Field | Meaning |
| --- | --- | --- |
| 0 | `MAJORID`, `MINORID` | 1, 1 |
| 8 | `EISDSIZE` | 36 |
| 12 | `SECSIZE` | section size in bytes |
| 16 | `VIRT_ADDR` | virtual address |
| 24 | `FLAGS` | see below |
| 28 | `VBN` | first block of the contents, counting from 1; 0 for demand-zero |
| 32 | `PFC`, `MATCHCTL`, `TYPE` | 0 (normal section) |

A descriptor never crosses a block boundary. A size of 0xFFFFFFFF, which is
what the 0xFF fill reads as, means "continue at the next block". A size of 0
ends the list: the end marker is 12 bytes, majorid, minorid and size all zero.

Flags used: `EXE` (0x800) code, `WRT` (0x8) writable, `CRF` (0x2) copy on
reference, `DZRO` (0x4) demand-zero, with no contents in the file. Global
sections (`GBL`, 0x1) belong to shareable images, which a reader rejects for
now.

**Changes:**

- A section is never both `EXE` and `WRT`. A reader rejects one that is.
- `VIRT_ADDR` is always a multiple of 64 KB, so the image loads with 4 KB,
  16 KB or 64 KB pages. That is also the Alpha linker's default (`/BPAGE=16`);
  here it is a requirement.
- Protection comes from the flags: `EXE` is read and execute, `WRT` is read and
  write, neither is read-only.
