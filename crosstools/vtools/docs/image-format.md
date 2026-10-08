# ARM64 executable image format

A vaxpunk executable image (EXE) is laid out like an OpenVMS Alpha image: a
header of 512-byte blocks, then the contents of each image section, each
starting on a block boundary. This document lists the changes from Alpha. The
code is `vms-obj/src/exe.rs`.

Status: executables, with a fixup section when the image is linked
`/RELOCATABLE`, so that a loader can move it, or when it calls a shareable
image; and shareable images, which always move, with their symbol vector
and global symbol table. Debug symbol tables keep their Alpha slots,
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
| 20 | `SYMDBGOFF` | offset of the EIHS, in a shareable image; else 0 |
| 24 | `IMGIDOFF` | offset of the EIHI |
| 28 | `PATCHOFF` | 0 |
| 32 | `IAFVA` | address of the fixup section; 0: none |
| 40 | `SYMVVA` | address of the symbol vector, in a shareable image; else 0 |
| 48 | `VERSION_ARRAY_OFF` | 0 |
| 52 | `IMGTYPE` | 1, executable, or 2, shareable (`EIHD$K_LIM`) |
| 56 | `SUBTYPE` | 0, native |
| 60 | `IMGIOCNT`, `IOCHANCNT` | 0 |
| 68 | `PRIVREQS` | all ones |
| 76 | `HDRBLKCNT` | number of header blocks |
| 80 | `LNKFLAGS` | `LNKNOTFR` (2) if there is no transfer address; `PICIMG` (8) if the image may move |
| 84 | `IDENT`, `SYSVER` | 0 |
| 92 | `MATCHCTL` | 0 |
| 96 | `SYMVECT_SIZE` | the symbol vector's entries, in a shareable image; else 0 |
| 100 | `VIRT_MEM_BLOCK_SIZE` | 16: sections are aligned to 2^16 bytes |
| 104 | `EXT_FIXUP_OFF`, `NOOPT_PSECT_OFF` | 0 |
| **112** | **`ARCH`** | **183, ARM64** |
| 510 | `ALIAS` | 0xFFFF |

**Change:** Alpha images have no architecture field, because only Alpha ran
them. vaxpunk puts one at offset 112, which Alpha leaves as fill, with the same
code as the object format (183, ELF's `EM_AARCH64`). A reader rejects any other
value. The EIHA, EIHI and, in a shareable image, the EIHS follow the fixed
part, 8-byte aligned; readers find them through the offsets.

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

An image linked `/RELOCATABLE` or `/SHAREABLE` (`docs/linker.md`) may be
loaded at another 64 KB-aligned base. Every address it stores in its own data
then has to move with it, and the fixup section lists where they are; its
`LNKFLAGS` has `PICIMG`. An image that calls a shareable image has one too,
movable or not: it lists those images, and where the image holds addresses
in them. It is one more image section, after the others, 64 KB aligned,
read-only, with the `FIXUPVEC` flag; `EIHD$Q_IAFVA` holds its address. A
loader reads it and may unmap it once the fixups are done. A reader takes it
out of the section list (`Image::fixups` and `Image::shareables` in
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
for an image's own addresses, and the quadword `.ADDRESS` list for the
addresses it holds in shareable images.

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
| 40 | `QDOTADROFF` | offset of the quadword `.ADDRESS` fixups; 0 for none |
| 44 | `LDOTADROFF`, `CODEADROFF`, `LPFIXOFF`, `CHGPRTOFF` | 0, and see below |
| 60 | `SHLSTOFF` | offset of the shareable image list; 0 for none |
| 64 | `SHRIMGCNT` | the entries in the list |
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
with contents. A reader rejects anything else, and relocation records in an
image without `PICIMG`.

### Shareable images an image calls

The shareable image list has a 64-byte `SHL$` entry for each image, in the
order of their indexes. vaxpunk fills in only the name, which the image
activator looks for in `SYS$SHARE:`, with `.EXE`:

| Offset | Field | vaxpunk value |
| --- | --- | --- |
| 0 | `BASEVA`, `SHLPTR`, `IDENT`, `PERMCTX` | 0 |
| 16 | `SIZE` | 64 |
| 17 | `FILL_1`, `FLAGS`, `ICB` | 0 |
| 24 | `IMGNAM` | the image's name, `.ASCIC`, 40 bytes |

The quadword `.ADDRESS` fixups are groups, one for each image with any: a
longword count and the image's index in the list, then that many pairs of
longwords, the offset of a quadword in this image (from its lowest section
address, as above) and the offset in the shareable image's symbol vector
of the entry the quadword gets. A zero count and index end them. The
offsets are in increasing order within a group.

**Changes:**

- Offsets count from the image's lowest section address, not from 0 (Alpha
  shareable images are linked at 0, where the two are the same). That also
  keeps the list from needing fixups of its own.
- `LW_MIN` and `LW_MAX` are new: with them, a loader can tell which
  displacements keep every longword address in range without scanning. They
  follow the Alpha fields, and `SIZE` counts them.
- vaxpunk writes no `LDOTADROFF`, `CODEADROFF` or `LPFIXOFF` lists, and
  a reader rejects them: an image reaches a shareable image only through
  quadwords (`docs/linker.md`).
- No change protection list. binutils makes each section with fixups writable
  in its descriptor and lists, from `CHGPRTOFF`, the protection to restore
  once the fixups are done. vaxpunk keeps each section's final protection in
  its descriptor instead: `$LINK$` stays read-only there, and the loader
  patches it before it applies protections.

## Shareable images

A shareable image (`IMGTYPE` 2) is always linked to move, `PICIMG` set, at
0 unless the linker was told otherwise; the image activator puts it where
it goes. It has no transfer address.

### Symbol vector

The procedures other images may call, in a quadword each, which holds the
procedure's address and moves with the image (a relocation fixup).
`EIHD$Q_SYMVVA` holds the vector's address and `EIHD$L_SYMVECT_SIZE` the
number of entries. An image that calls the procedure in entry i gets the
quadword at `SYMVVA` + 8i, moved, from the image activator.

**Change:** Alpha's entries are 16 bytes, two quadwords, since a procedure
value there is a procedure descriptor and its code address is the second
quadword. vaxpunk calls the code address (`docs/linker.md`), and has only
procedure entries, so an entry is one quadword.

### Global symbol table (`EIHS$`, GST)

The image symbol table header, 32 bytes at `EIHD$L_SYMDBGOFF`, says where
the GST is:

| Offset | Field | vaxpunk value |
| --- | --- | --- |
| 0 | `MAJORID`, `MINORID` | 1, 1 |
| 8 | `DSTVBN`, `DSTSIZE` | 0: no debug symbol table |
| 16 | `GSTVBN` | the GST's first block, counting from 1 |
| 20 | `GSTSIZE` | how many records it has |
| 24 | `DMTVBN`, `DMTBYTES` | 0 |

The GST comes after the section contents, from a block boundary: an object
module (`docs/object-format.md`), as the Alpha linker writes it. Its module
header has the image's name and ident; a GSD record defines the absolute
psect, `.$$ABS$$.` (`PIC`, `LIB`, `RD`, empty); further GSD records have an
`EGSD$C_SYMG` subrecord (type 8, `EGST$`) for each entry of the symbol
vector; and an end-of-module record ends it.

| Offset | Field | vaxpunk value |
| --- | --- | --- |
| 0 | `GSDTYP`, `GSDSIZ` | 8, the subrecord's size |
| 4 | `DATYP`, `TEMP` | 0 |
| 6 | `FLAGS` | `DEF`, `UNI`, `REL`, `NORM`: a procedure others may call |
| 8 | `VALUE` | the entry's offset in the symbol vector |
| 16 | `LP_1`, `LP_2` | the procedure's address, as linked |
| 32 | `PSINDX` | 0 |
| 36 | `NAME` | `.ASCIC` |

The linker reads the GST to link against the image. A reader rejects an
entry that isn't a procedure's, and entries not in the vector's order.

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
