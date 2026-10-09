# velf: C objects into object modules

`velf` turns the ELF relocatable object a C compiler writes for AArch64
into an object module (`docs/object-format.md`), which `vlib` and `vlink`
take as they take MACRO-32's and BLISS-64's. vaxpunk has no C compiler of
its own: a stock gcc compiles the C, with the flags below, and `velf`
converts its output. `examples/c` builds a BLISS-64 program that calls a
C library this way, and its README says what crosses between the two
calling conventions and what doesn't.

```
velf [/OBJECT=file | -o file] FILE.o
```

The object file defaults to the input's name with `.obj`. The module is
named after the object file, in upper case. A module name has at most 31
characters, so `/OBJECT` names one whose C file has a longer name.

## The compiler's flags

`velf::GCC_FLAGS` holds them, and the tests and `examples/c/Justfile`
use them:

| Flag | Why |
| --- | --- |
| `-ffreestanding` | No hosted C library. gcc may still call `memcpy`, `memset`, `memmove` and `memcmp`, which another module must define |
| `-fno-pic`, `-fno-pie` | Code reaches data with `ADRP` and `ADD`, not through a global offset table, which velf has no relocations for |
| `-fno-common` | A tentative definition is a `$BSS$` symbol, not a common one, which velf refuses |
| `-ffixed-x18` | x18 is the VAX stack pointer (DESIGN-0004). The PAL writes below the lower of `sp` and x18 when it delivers an interrupt, so C must never touch it |
| `-mgeneral-regs-only` | The PAL's exception frame keeps x0-x30 and `sp`, not the FP and SIMD registers (ADR-0023), so code an AST or a handler runs must leave them alone. gcc uses them even to copy memory |
| `-fno-omit-frame-pointer` | VMS finds frames through x29, for condition handling and `$UNWIND`. A C routine that used x29 as a scratch register would break the chain for a routine it calls back. Leaf routines still have no frame record, which is harmless: they call nobody |
| `-fno-stack-protector` | No `__stack_chk_guard` or `__stack_chk_fail` |
| `-fno-asynchronous-unwind-tables`, `-fno-unwind-tables` | No `.eh_frame`: VMS walks frames through their descriptors (velf drops `.eh_frame` anyway) |
| `-mbranch-protection=none` | No pointer authentication or BTI instructions, which Ubuntu's gcc adds by default |
| `-mno-outline-atomics` | Atomic operations inline, not calls to libgcc's `__aarch64_*` helpers, which Linux gcc makes by default |

## What goes where

**Sections become psects**, with DEC C's names. Each section's place in
its psect follows its alignment; the psect's alignment is the largest.

| ELF section | Psect | Attributes |
| --- | --- | --- |
| `.text`, `.text.*` | `$CODE$` | `PIC REL SHR EXE` |
| `.rodata`, `.rodata.*`, without addresses | `$READONLY$` | `PIC REL SHR RD` |
| the same, with addresses (`ABS64` or `ABS32` relocations) | `$READONLY_ADDR$` | `REL RD`: not PIC, since a moved image patches the addresses (`docs/linker.md`) |
| `.data`, `.data.*` | `$DATA$` | `REL RD WRT` |
| `.bss`, `.bss.*` | `$BSS$` | `REL RD WRT NOMOD` |

Sections that aren't loaded, notes, `.eh_frame` and empty sections are
dropped. Any other loaded section is an error.

**Symbols.** Each global or weak definition is a symbol definition,
`DEF REL`, `WEAK` for a weak one; each undefined one, a reference. Names
are folded to upper case, as DEC C folds them by default; two names that
differ only in case are an error. Names may be as long as the object
format allows, 255 characters, and vbliss, vlib and vlink take them.
Common and absolute symbols are errors.

**Relocations become stores.** Each pushes the target, `STA_GBL` of a
global symbol, with `STA_QW` and `OPR_ADD` for an addend, or `STA_PQ` of a
local symbol's psect and offset, then stores:

| AArch64 relocation | Store |
| --- | --- |
| `CALL26`, `JUMP26` | `STO_A64_JUMP26` |
| `CONDBR19`, `LD_PREL_LO19` | `STO_A64_BRANCH19` |
| `TSTBR14` | `STO_A64_BRANCH14` |
| `ADR_PREL_LO21` | `STO_A64_ADR` |
| `ADR_PREL_PG_HI21`, `ADR_PREL_PG_HI21_NC` | `STO_A64_ADRP` |
| `ADD_ABS_LO12_NC` | `STO_A64_ADD_LO12` |
| `LDST8`, `16`, `32`, `64`, `128_ABS_LO12_NC` | `STO_A64_LDST8_LO12` to `LDST128_LO12` |
| `ABS64`, `ABS32` | `STO_QW`, `STO_LW` |

An instruction store gets the instruction with the relocated field
cleared. Anything else, such as the large code model's `MOVW` relocations,
a global offset table's or thread-local storage's, is an error.

ponytail: no debugger or traceback records, and no unwind table for C's
frames, which DESIGN-0004 leaves for when another compiler's code
arrives; until then no condition may reach a C frame (`examples/c`).
