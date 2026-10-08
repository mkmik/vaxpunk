# ARM64 executable image format

A vaxpunk executable image (EXE) is laid out like an OpenVMS Alpha image: a
header of 512-byte blocks, then the contents of each image section, each
starting on a block boundary. This document lists the changes from Alpha. The
code is `vms-obj/src/exe.rs`.

Status: only what a statically linked executable needs is produced, plus a
fixup section when the image is linked `/RELOCATABLE`, so that a loader can
move it. Shareable images and debug symbol tables keep their Alpha slots,
unused.

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
| 32 | `IAFVA` | address of the fixup section; 0: none, the image can't move |
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
reference, `DZRO` (0x4) demand-zero, with no contents in the file, and
`FIXUPVEC` (0x40) for the fixup section. Global sections (`GBL`, 0x1) belong
to shareable images, which a reader rejects for now.

**Changes:**

- A section is never both `EXE` and `WRT`. A reader rejects one that is.
- `VIRT_ADDR` is always a multiple of 64 KB, so the image loads with 4 KB,
  16 KB or 64 KB pages. That is also the Alpha linker's default (`/BPAGE=16`);
  here it is a requirement.
- Protection comes from the flags: `EXE` is read and execute, `WRT` is read and
  write, neither is read-only.

## Fixup section (`EIAF$`)

An image linked `/RELOCATABLE` (`docs/linker.md`) may be loaded at another
64 KB-aligned base. Every address it stores in its own data then has to move
with it, and the fixup section lists where they are. It is one more image
section, after the others, 64 KB aligned, read-only, with the `FIXUPVEC` flag;
`EIHD$Q_IAFVA` holds its address. A loader reads it and may unmap it once the
fixups are done. A reader takes it out of the section list (`Image::fixups` in
`vms-obj`). `vdump` shows each fixup with the value there, and, given the link
map with `--map`, the psect it is in.

### What the Alpha lists mean

The layout comes from GNU binutils: `include/vms/eiaf.h` for the header,
`alpha_vms_build_fixups` in `bfd/vms-alpha.c` for what its linker writes, and
`evax_bfd_print_image_fixups` there for what it reads. Two families of lists
look alike and aren't:

- The `.ADDRESS` fixups (`EIAF$L_QDOTADROFF`, `LDOTADROFF`) are for addresses
  in **other** images. They are grouped by shareable image, each group a
  count and the image's index in the shareable image list, and each entry is
  a pair: the offset in this image (from its base) of the quadword or
  longword to patch, and what it refers to in that image (a symbol vector
  entry). binutils writes the quadword list, for references to symbols in
  shareable images.
- The relocation fixups (`EIAF$L_QRELFIXOFF`, `LRELFIXOFF`) are the image's
  **own** addresses, which change by however far the image itself moves.
  binutils decodes them but never writes them, since its images don't move.

So vaxpunk uses the relocation fixups, in the record format binutils decodes,
and leaves the `.ADDRESS` lists for shareable images.

### Layout

The header, then the quadword records, then the longword records. Offsets in
the header count from the start of the section.

| Offset | Field | vaxpunk value |
| --- | --- | --- |
| 0 | `MAJORID`, `MINORID` | 0, 0, as binutils writes |
| 8 | `IAFLINK`, `FIXUPLNK` | 0: for the image activator's own use |
| 24 | `SIZE` | 92: the size of the header |
| 28 | `FLAGS` | 0 |
| 32 | `QRELFIXOFF` | offset of the quadword relocation records; 0 for none |
| 36 | `LRELFIXOFF` | offset of the longword relocation records; 0 for none |
| 40 | `QDOTADROFF`, `LDOTADROFF`, `CODEADROFF`, `LPFIXOFF`, `CHGPRTOFF`, `SHLSTOFF`, `SHRIMGCNT` | 0: shareable images, and see below |
| 68 | `SHLEXTRA`, `PERMCTX`, `BASE_VA`, `LPPSBFIXOFF` | 0 |
| **84** | **`LW_MIN`** | **the lowest address a longword fixup holds, signed** |
| **88** | **`LW_MAX`** | **the highest one** |

A list of relocation records ends with a zero count. Each record is:

| Size | Field |
| --- | --- |
| 4 | bit count, a multiple of 32 |
| 4 | base: an offset from the image's lowest section address |
| bit count / 8 | bits, 32 to a longword: bit *i* of longword *w* stands for the slot at base + (32*w* + *i*) × 8 for quadwords, × 4 for longwords |

A set bit says that the quadword or longword there holds an address of the
image. vlink starts a new record when the next address isn't a whole number
of slots after the base, or when more than 64 empty slots would come first.
Entries are in increasing order, never twice, and always inside a section
with contents. A reader rejects anything else, and any fixups for shareable
images.

**Changes:**

- Offsets count from the image's lowest section address, not from 0 (Alpha
  shareable images are linked at 0, where the two are the same). That also
  keeps the list from needing fixups of its own.
- `LW_MIN` and `LW_MAX` are new: with them, a loader can tell which
  displacements keep every longword address in range without scanning. They
  follow the Alpha fields, and `SIZE` counts them.
- No change protection list. binutils makes each section with fixups writable
  in its descriptor and lists, from `CHGPRTOFF`, the protection to restore
  once the fixups are done. vaxpunk keeps each section's final protection in
  its descriptor instead: `$LINK$` stays read-only there, and the loader
  patches it before it applies protections.

## Moving an image

A loader moves an image with a fixup section in five steps. `vms-obj` provides
steps 1, 2 and 4 as `Fixups::displacement` and `Fixups::apply`, which work on
byte buffers and build without `std`.

1. Choose a new base for the lowest section: a multiple of 64 KB, and, if there
   are longword fixups, one that keeps `LW_MIN` and `LW_MAX` plus the
   displacement within a signed longword.
2. The displacement D is the new base minus the lowest section address.
3. Map each section at its address plus D, writable for now.
4. Add D to each quadword the quadword records name. Add D to each longword
   the longword records name, and check that it still fits in a signed
   longword.
5. Apply each section's protection.

D is a multiple of 64 KB, so the low 16 bits of every address stay the same;
the linker counts on that for `:lo12:` and `MOVZ`/`MOVK` of the low chunk
(`docs/linker.md`).
