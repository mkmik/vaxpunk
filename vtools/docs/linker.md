# vlink: the linker

`vlink` links object modules (`docs/object-format.md`) into an executable
image (`docs/image-format.md`) at a fixed base address.

```
vlink [/EXE=file] [/MAP[=file]] [/BASE=address] [/TRANSFER=symbol] FILE.OBJ...
vlink [-o file] [-m file] [--base address] [--transfer symbol] FILE.OBJ...
```

The image defaults to the first input's name with `.exe`, the map to the image's
with `.map`. Addresses are decimal, `0x` or `%X` hex. The default base is
`%X10000`, as on VMS. Messages are VMS-style (`%VLINK-E-UDFSYM, ...`); any error
means no image is written.

Status: work order steps 6 and 7. Object libraries and procedure descriptors
come later.

## What it does

1. **Collects psects by name** across modules, in the order they appear.
   Attributes must agree. `CON` contributions are concatenated, each at its own
   alignment; `OVR` contributions share one address. A psect's alignment is its
   largest contribution's.
2. **Groups psects into image sections by protection**, in this order: code
   (`EXE`), read-only data (`NOWRT`), writable data (`WRT`), demand-zero (`WRT`
   and `NOMOD`, as `$BSS$` is). Each image section starts on a 64 KB boundary,
   from the base up. Absolute psects (`ABS`) take no space; their symbols are
   constants.
3. **Resolves symbols.** A symbol defined in two modules is an error unless one
   definition is weak. A reference to an undefined symbol is an error naming
   the modules that use it, unless the reference is weak, and then the symbol
   is 0.
4. **Runs each module's TIR commands** against the final addresses: a stack of
   64-bit values and a location counter. Stores are range-checked. So are ARM64
   instruction patches: a target out of reach, or a `:lo12:` address not aligned
   to the access size, is an error naming the module and location. Nothing is
   silently truncated, and there are no range-extension veneers. Data stored
   into a demand-zero psect is an error.
5. **Picks the transfer address** from `/TRANSFER`, or else from the first
   module whose end-of-module record has one.

## The map

The map has the sections of VMS `LINK/MAP`: object module synopsis, image
section synopsis, program section synopsis (each psect, then each module's
contribution), symbols by name, symbols by value, and an image synopsis with
the transfer address. Addresses are 16 hex digits throughout.
