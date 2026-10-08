# vlink: the linker

`vlink` links object modules (`docs/object-format.md`) into an executable
image (`docs/image-format.md`) at a fixed base address, taking modules from
object libraries (`docs/library-format.md`) as they are needed. With
`/RELOCATABLE`, the image also says how to move it elsewhere.

```
vlink [/EXE=file] [/MAP[=file]] [/BASE=address] [/TRANSFER=symbol] [/RELOCATABLE] FILE...
vlink [-o file] [-m file] [--base address] [--transfer symbol] [--relocatable] FILE...
```

A FILE is an object module or an object library, `LIB.OLB/LIBRARY` as on VMS,
or just `LIB.OLB`: vlink knows a library when it sees one.

The image defaults to the first input's name with `.exe`, the map to the image's
with `.map`. Addresses are decimal, `0x` or `%X` hex. The default base is
`%X10000`, as on VMS. Messages are VMS-style (`%VLINK-E-UDFSYM, ...`); any error
means no image is written.

Status: work order steps 6, 7 and 9. Procedure descriptors come later.

## What it does

1. **Collects psects by name** across modules, in the order they appear.
   Attributes must agree. `CON` contributions are concatenated, each at its own
   alignment; `OVR` contributions share one address. A psect's alignment is its
   largest contribution's.
2. **Groups psects into image sections by protection**, in this order: code
   (`EXE`), read-only data (`NOWRT`), writable data (`WRT`), demand-zero (`WRT`
   and `NOMOD`, as `$BSS$` is). Each image section starts on a 64 KB boundary,
   from the base up. Absolute psects (`ABS`) take no space; their symbols are
   constants, and every module's contribution starts at 0, as in an overlaid
   psect.
3. **Resolves symbols.** A symbol defined in two modules is an error unless one
   definition is weak. A reference to an undefined symbol is an error naming
   the modules that use it, unless the reference is weak, and then the symbol
   is 0.
4. **Runs each module's TIR commands** against the final addresses: a stack of
   64-bit values, each with its weight (see *Movable images*), and a location
   counter. Stores are range-checked. So are ARM64
   instruction patches: a target out of reach, or a `:lo12:` address not aligned
   to the access size, is an error naming the module and location. Nothing is
   silently truncated. A `B` or `BL` that can't reach a fixed address, as a
   call from an image in P0 to a system service in S0, is the one
   exception: vlink notes the address and links again with a veneer for it
   in `$VENEER$`, a code psect of a module of its own, `$VENEERS`:
   `ldr x16, 8; br x16` and the address, which DESIGN-0004 lets a linker put
   between a call and its target. A target that moves with the image gets no
   veneer, and stays an error. Data stored into a demand-zero psect is an
   error.
5. **Picks the transfer address** from `/TRANSFER`, or else from the first
   module whose end-of-module record has one.

## Object libraries

vlink searches a library where it appears among the inputs, the way VMS LINK
does. For each symbol that the modules so far reference strongly and none
defines, it takes from the library the module that the symbol index names, and
it keeps going with the references of the modules it took, until the library
has nothing more to give. So a library comes after the modules that need it,
and a module it gives can need others from the same library. A weak reference
alone doesn't take a module; it is resolved if some module taken for another
reason defines the symbol, and is 0 otherwise. Modules taken from a library
show the library as their file in the map.

`vlib` builds libraries:

```
vlib [/CREATE] LIBRARY.OLB FILE.OBJ...
vlib /LIST [/NAMES] LIBRARY.OLB
```

(`--create`, `--list` and `--names` work too.) Each module of each object file
goes in, replacing a module of the same name, as `LIBRARY/REPLACE` does.
`/CREATE` starts a new library instead of updating one. The symbol index gets
each module's strong global definitions; a symbol already there for another
module stays with that module, with a warning. `/LIST` prints the module
names, and with `/NAMES` the symbols of each.

## The map

The map has the sections of VMS `LINK/MAP`: object module synopsis, image
section synopsis, program section synopsis (each psect, then each module's
contribution), symbols by name, symbols by value, and an image synopsis with
the transfer address. Addresses are 16 hex digits throughout.

After the program sections, if the image stores addresses of its own, a
fixups section lists each: its address, whether a quadword or a longword, the
module that stored it, and where, as psect + offset in that module's
contribution and the nearest global label before it.

## Movable images

As on VMS, code reaches its own image PC-relative, and everything else through
addresses stored in data, such as a linkage section. An image is linked at a
fixed base and runs there; only an image linked `/RELOCATABLE` may be moved by
its loader, which patches the addresses stored in it, the fixups
(`docs/image-format.md` says how). The linker knows which values are
addresses, and fails the link if something can't move.

### Weights

The loader moves an image by D, a multiple of 64 KB. Then each value on the
linker's stack changes by a known multiple of D, its weight k: the value is a
constant plus k × D. Some values aren't of that form, such as an address
shifted right; their weight is unknown.

| Source or operator | Weight |
| --- | --- |
| `STA_LW`, `STA_QW` | 0 |
| `STA_PQ` of a `REL` psect | 1 |
| `STA_PQ` of an `ABS` psect | 0 |
| `STA_GBL` of a symbol defined with `EGSY$V_REL` | 1 |
| `STA_GBL` of any other symbol, or of an undefined weak one | 0 |
| `OPR_ADD`, `OPR_SUB` | k₁ + k₂, k₁ − k₂ |
| `OPR_NEG` | −k |
| `OPR_MUL` | k × c, if the other side is a constant c (weight 0) |
| any other operator | 0 if every operand is 0, otherwise unknown |

### What each store needs

Where a store goes, P, has weight 1, so a PC-relative field, S − P, has
weight k − 1: it is safe exactly when k is 1. The same goes for data:
`target - .` needs nothing.

| Store | k = 0 | k = 1 | anything else |
| --- | --- | --- | --- |
| `STO_QW`, `STO_OFF`, `STO_GBL`, `STO_CA` | nothing | quadword fixup | can't move |
| `STO_LW` | nothing | longword fixup | can't move |
| `STO_W`, `STO_B`, `STO_IMMR` | nothing | can't move | can't move |
| `JUMP26`, `BRANCH19`, `BRANCH14`, `ADR`, `ADRP` | can't move: the target stays, the code moves | nothing | can't move |
| `ADD_LO12`, `LDST*_LO12`, `MOVW_G0_NC` | nothing | nothing: the low 16 bits never change | can't move |
| `MOVW_G0`, and `MOVW_G1` to `G3` with their `_NC` forms | nothing | can't move | can't move |

`MOVW_G0` promises that the rest of the address is zero, which stops being true
once the image moves, so it can't move with an address even though its 16 bits
don't change.

A fixup is a write the loader makes into the image. It may only land in a
`NOPIC` psect: `PIC` means that a psect can be shared unchanged, as `$CODE$`,
`$LITERAL$` and `$READONLY$` are. `$LINK$` is `NOPIC` and `NOWRT`, as on
Alpha: the loader patches it before it makes it read-only.

A longword fixup ties the image to the low 2 GB: an address in a longword is
sign-extended when it is loaded, as on VMS, so it must fit in 32 signed bits,
where the image is linked and wherever it moves.

What counts is what the image holds in the end. A store over an address, or
over something that couldn't move, replaces it. What a store leaves of part of
an address is neither an address nor a constant, and can't move.

### Diagnostics

The linker tracks weights on every link. Each problem is reported once, with
the module, psect, offset and nearest global label:

| Problem | Without `/RELOCATABLE` | With `/RELOCATABLE` |
| --- | --- | --- |
| A fixup in a `PIC` psect | `%VLINK-W-NOTPIC` | `%VLINK-E-NOTPIC` |
| A store that can't move, from the table | nothing | `%VLINK-E-NORELOC` |
| A longword address at or above 2 GB | `%VLINK-E-TRUNC` | `%VLINK-E-TRUNC` |

```
%VLINK-E-NOTPIC, address in a PIC psect needs a fixup, at $CODE$ + %X4 (TABLE) in module MAIN (main.obj)
%VLINK-E-NORELOC, STO_A64_MOVW_G1 of an address can't move, at $CODE$ + %X0 (START) in module MAIN (main.obj)
```

`/RELOCATABLE` also writes the fixup section and reports
`%VLINK-I-FIXUPS, n quadword and m longword fixups`. The image is still linked
at `/BASE` and runs there; moving it is up to the loader. Without it, the image
is exactly what vlink made before movable images existed.

`vdump --weights` shows the weight of each store in an object module, which
explains a `NOTPIC` or `NORELOC` from the linker.

### Reaching other data from code

Code reaches an address outside its psect through a linkage slot, in the
Alpha way, and `vtools/lib/pic.mlb` has a macro that makes one:

```
        .PSECT  $LINK$
L_BUF:  .ADDRESS BUF                    ; a quadword fixup, in a NOPIC psect
        .PSECT  $CODE$
        adrp    x0, L_BUF               ; PC-relative: moves with the code
        ldr     x0, [x0, #:lo12:L_BUF]  ; the low 12 bits never change
```

Within one image, `adrp` and `add` to the target itself work just as well and
skip the load; the slot matters once the target can be in another image.

### Procedure descriptors

Procedure descriptors keep Alpha's layout and its absolute fields, and live in
`$LINK$` or another `NOPIC` psect, where their addresses are ordinary quadword
fixups; the linker has nothing special for them. A function pointer is the
address of a descriptor, never of code: an indirect call loads the entry
address from the descriptor, then `BLR`. No register holds the procedure value
on entry, as `R27` does on Alpha; code reaches its own image with `ADRP`. What
else a descriptor holds belongs to the calling standard. `tests/run/callptr`
calls through descriptors of the minimal kind, flags and then the entry address
at offset 8.
