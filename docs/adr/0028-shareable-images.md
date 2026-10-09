# ADR-0028 — Shareable images are Alpha's, linked against their symbol vector and called through linker veneers; the image activator maps a copy into each process, and LIBRTL.EXE holds the LIB$ routines

Oct 8, 2026 · @Marko Mikulicic

Proposed. vaxpunk has shareable images in OpenVMS Alpha's format: a
symbol vector of procedures, a global symbol table to link against, and
the fixup section's shareable image list and `.ADDRESS` fixups. vlink
makes one with `/SHAREABLE` and a `SYMBOL_VECTOR` option, and links an
image against one. The image calls each procedure through a veneer that
jumps through a quadword the image activator sets. The image activator
finds each shareable image an image calls in `SYS$SHARE:`, maps a copy
past the image in P0, moves it with its own fixups, and fills in the
image's quadwords from its symbol vector, as VMS does for a shareable
image that isn't installed. `LIBRTL.EXE`, in `SYS$LIBRARY:`, holds
`LIB$PUT_OUTPUT`, `LIB$GET_INPUT` and the condition handling routines,
and every program but DCL calls it. Item 20 of
[PRD-0003](../prd/0003-multi-user-vms.md)'s backlog, without the sharing
of pages, which waits for global sections.

## Context

Every program on the system disk links `sysexe/lib/` in, the run-time
routines with the rest (`build.rs`), so each has its own copy, on disk
and in memory. On OpenVMS the `LIB$` routines are in `LIBRTL.EXE`, a
shareable image: `IMAGELIB.OLB` on the Alpha V8.4-2L1 CD lists
`LIB$GET_FOREIGN` and the others for it. The `CLI$` routines aren't
there: `STARLET.OLB` has `CLI$GET_VALUE`, `CLI$PRESENT` and
`CLI$DCL_PARSE` as object modules, which LINK puts in each image.

[PRD-0001](../prd/0001-vtools.md) left room for shareable images in the
image header and the fixup section, and vlink could already make an image
that moves (`/RELOCATABLE`): it records the addresses an image holds,
and fails the link on anything a loader couldn't move. The image
activator mapped one image, at its link address.

A VMS image reaches a shareable image only through its symbol vector: the
linker resolves a reference to an offset in the vector, from the
shareable image's GST, and the image activator sets the referencing
quadword from the vector when it maps the image. On Alpha the quadwords
are in the caller's linkage section, which its code loads procedure
descriptors from.

vaxpunk has no linkage sections ([ADR-0023](0023-calling-standard.md)).
vmacro compiled `CALLS G^name` as `adrp`, `add` and `blr`, which reach
±4 GB from the code, and BLISS-64 calls with `BL`; both name the target
in the instruction. For BLISS-64, vlink already put a veneer between a
`BL` and a fixed address out of its reach (ADR-0026).

Sharing a shareable image's pages between processes needs more than
this: a PFN maps at one address at a time
([DESIGN-0001](../design/0001-pal-interface.md)), and `$CRMPSC` and
`$MGBLSC` are stubs. VMS maps a shareable image that isn't installed
privately, a copy in each process, and installs those it shares.

## Decision

1. **The format is Alpha's** (`crosstools/vtools/docs/image-format.md`). A
   shareable image has `IMGTYPE` 2, `PICIMG` set, and a fixup section
   that moves it. Its symbol vector, at `EIHD$Q_SYMVVA`, is a quadword
   for each procedure, holding its address; Alpha's entries are 16
   bytes. Its GST is an object module of `EGSD$C_SYMG` entries, name and
   vector offset, which the EIHS locates. An image that calls one lists
   it in its fixup section's shareable image list (`SHL$`), and names
   each quadword it holds an address there in by a quadword `.ADDRESS`
   fixup: its offset in the image and the symbol vector offset.
2. **vlink makes one with `/SHAREABLE`** (`crosstools/vtools/docs/linker.md`),
   linked at 0 to move, with its symbol vector in an options file as VMS
   LINK takes it: `SYMBOL_VECTOR=(LIB$PUT_OUTPUT=PROCEDURE, ...)`.
   Procedures only. An entry keeps its place, and new ones go at the end.
3. **An image calls a shareable image's procedure through a veneer.**
   A shareable image among vlink's inputs defines its vector's
   procedures. For each one the image uses, vlink adds a veneer to
   `$VENEER$`, `adrp`/`ldr`/`br x16` through a quadword slot in
   `$LINK$`, and the slot gets the `.ADDRESS` fixup. The procedure's name
   stands for its veneer, so `BL`, `adrp` with `add`, and `.ADDRESS` of
   it all work, and the image's code holds no address of another image.
4. **`G^` calls are `BL`.** vmacro compiles `CALLS`, `CALLG`, `JSB` and
   `JMP` to `G^name` as `bl` and `b`, as it does without `G^`; vlink puts
   a veneer in when the target is too far or in another image. In an
   image that moves, every `BL` to a fixed address gets one, since the
   distance changes, so `LIBRTL.EXE` calls the system services as any
   image does.
5. **The image activator maps a copy into the process.** `IMG$ACTIVATE`
   maps the image's sections writable by the kernel. For each entry of
   its shareable image list, `SHRACT` reads `SYS$SHARE:name.EXE`
   (`FIL$OPENSHR`), maps it at the next 64 KB past the end of P0, adds the
   distance to each address its relocation records name, and protects it
   for user mode. `SHRLIST` then sets each of the image's slots for that
   image from its symbol vector, and only then is the image protected.
   Image rundown frees the copy with the rest of P0. Only an image in P0
   may call a shareable image, and a shareable image may not call
   another.
6. **`LIBRTL.EXE` holds the run-time library's routines**:
   `LIB$PUT_OUTPUT`, `LIB$GET_INPUT`, `LIB$ESTABLISH`, `LIB$REVERT`,
   `LIB$SIG_TO_RET`, `LIB$SIGNAL` and `LIB$STOP`, from
   `sysexe/librtl/`, with the symbol vector `librtl.opt` gives. It is in
   `[SYSLIB]`, which `SYS$LIBRARY` and `SYS$SHARE` name, at boot
   `SYS$SYSDEVICE:[SYSLIB]` and after `SYSTARTUP_VMS.COM`
   `SYS$SYSROOT:[SYSLIB]`, as on VMS. `PUT_LINE`, behind the `PRINT`
   macros, stays in `sysexe/lib/` and calls `LIB$PUT_OUTPUT`, so every
   program but DCL calls `LIBRTL.EXE`. The `CLI$` routines stay linked
   in, as `STARLET.OLB` links them on VMS, and so does
   `LIB$GET_FOREIGN`, which calls `CLI$GET_VALUE` in the image. DCL, in
   P1, links `LIBRTL`'s modules in.

## Alternatives considered

| Option | Why not |
| --- | --- |
| Wait for global sections, and make shareable images shared from the start | The linker, the format and the activator are the same work either way, and VMS maps a shareable image that isn't installed privately too. Sharing pages is a PAL change of its own. |
| Link each shareable image at a fixed address of its own, as the executive is in S0 | No fixups, but every shareable image would need a range no other one and no image uses, chosen by hand. VMS moves them, and vlink could already make an image that moves. |
| Resolve by name when the image is activated, as ELF's dynamic linker does | The activator would read both symbol tables and compare strings. VMS resolves to a vector offset at link time, and an offset survives a new version of the shareable image that keeps its vector. |
| Patch the call, or the veneer's own literal, with the procedure's address | A fixup in code, which a `PIC` psect may not have: the code couldn't be shared once global sections come, and a `BL` can't reach every address. |
| Alpha's linkage pairs: the caller's code loads the address from its linkage section | Every call site in vmacro and vbliss would change to load and `blr`, and the calls to the image's own routines would pay too. A veneer costs only the calls into another image, and DESIGN-0004 already sets x16 and x17 aside for it. |
| Keep `G^` as `adrp`, `add` and `blr`, and have vlink send those to a veneer | vlink can't tell a call's `adrp` from a data reference's. A `BL` says it is a call. |
| Put the `CLI$` routines in `LIBRTL.EXE` too | They aren't there on VMS, and their state is the image's: the parse it was given. The CLI parser linked into each image is ADR-0017's, and goes when the parse moves to DCL. |
| Name the shareable image `VAXPUNK$RTL` or similar | It holds VMS's routines under VMS's names; a program written for VMS links against `LIBRTL`. |

## Consequences

**What gets harder.**
- Activating an image reads `LIBRTL.EXE` too, and maps 3 more sections:
  slower, and nonpaged pool holds both files while it does.
- Each process still has its own copy of `LIBRTL.EXE`'s pages, as of
  every image. Memory is saved only once global sections share them.
- An address an image takes of a `LIBRTL` procedure is its veneer's, so
  two images' addresses of the same procedure differ. Nothing compares
  them yet.
- The image activator doesn't check `GSMATCH`, or a shareable image's
  ident, so an image linked against another vector can call the wrong
  entry. The images on the system disk are all linked together.
- A shareable image can't export data, call another shareable image, or
  be called from DCL, which lives in P1.

**What stays easy.**
- A program calls `LIB$PUT_OUTPUT` and the others as it did; the linker
  and the activator do the rest.
- A new routine goes in `sysexe/librtl/` and at the end of
  `librtl.opt`.
- `vdump` shows a shareable image's symbol vector, and the shareable
  images an image calls with their slots.

**Follow-ups:**
- Global sections, `$CRMPSC` and `$MGBLSC`, on a frame capability for
  each extra mapping of a PFN, then `INSTALL /SHARED` and the known file
  list, so that processes share `LIBRTL.EXE`'s pages.
- `DATA` entries in the symbol vector, reached through a slot as the
  procedures are.
- More of VMS's `LIBRTL`: `LIB$GET_FOREIGN` once the `CLI$` routines
  ask DCL, `LIB$FIND_IMAGE_SYMBOL`, and the string and time routines as
  programs need them.
- The image activator translating the shareable image's name as a logical
  name first, as VMS's does, and checking `GSMATCH`.
