# ARM64 object language

The vaxpunk object module format (OBJ) is the OpenVMS Alpha object language
with the changes ARM64 forces, and nothing else changed. This document lists
those changes. Anything it doesn't mention works exactly as described in the
Alpha specification below. Compilers and assemblers are written against this
document plus that specification.

Status: `vasm` writes this format and `vlink` reads it. `vlink` implements all
the ARM64 store commands, tested at their range limits. Procedure descriptors
and linkage wait for the calling standard.

## Sources

The Alpha object language is specified in Appendix B, "Alpha Object Language",
of:

| | |
| --- | --- |
| Title | OpenVMS Linker Utility Manual |
| Order number | AA–PV6CD–TK |
| Edition | April 2001, OpenVMS Alpha Version 7.3 / OpenVMS VAX Version 7.3 |
| URL | <https://zx.net.nz/mirror/h71000.www7.hp.com/doc/73final/documentation/pdf/OVMS_73_LINKER_UTIL.PDF> |
| SHA-256 | `babfce6428f30431bae709b148426bd6a845176c5f6381a535f9b2f77992366a` |

Later editions (for example BA554–90004, July 2006, for V8.3) dropped the
appendix and point back to this one. The manual is Compaq/HP copyright, so the
repository keeps only this reference and our own notes.

GNU binutils implements the same format (`bfd/vms-alpha.c`, and the record
layouts in `include/vms/*.h`) and is a useful cross-check. It is GPL-3: learn
layouts from it, never copy code. It disagrees with the manual on store commands
62, 64 and 65; the manual wins, and those codes are unused here anyway.

## Conventions

Integers are little-endian. Names use the Alpha spelling, `EOBJ$C_EMH`, and
fields are called by their Alpha names (`EGPS$W_FLAGS` is the 2-byte flags field
of a psect definition). A counted string (ASCIC) is a length byte followed by
that many characters. Symbol and psect names are folded to upper case.

## Framing

An OBJ file is a sequence of records, one object module after another. Every
record starts with a 2-byte type (`EOBJ$W_RECTYP`) and a 2-byte size
(`EOBJ$W_SIZE`) that counts the whole record, header included. The maximum
record size is 8192 bytes (`EOBJ$C_MAXRECSIZ`).

**Change:** on VMS an object file is an RMS variable-length record file, so each
record is also preceded by an RMS record length. vaxpunk files have no RMS: records
are simply concatenated, and the size field alone delimits them. This is the
layout binutils calls "native".

## Record types

Unchanged:

| Value | Name | Use |
| --- | --- | --- |
| 8 | `EOBJ$C_EMH` | module header |
| 9 | `EOBJ$C_EEOM` | end of module |
| 10 | `EOBJ$C_EGSD` | global symbol directory |
| 11 | `EOBJ$C_ETIR` | text, information and relocation |
| 12 | `EOBJ$C_EDBG` | debugger information (not produced yet) |
| 13 | `EOBJ$C_ETBT` | traceback information (not produced yet) |

The ordering rules are the Alpha ones: MHD header first, LNM header second, at
least one GSD record, EEOM last.

## Module header (`EOBJ$C_EMH`)

Subtypes 0–6 (MHD, LNM, SRC, TTL, CPR, MTC, GTX) are unchanged. The MHD layout:

| Offset | Size | Field | vaxpunk value |
| --- | --- | --- | --- |
| 0 | 2 | `EOBJ$W_RECTYP` | 8 |
| 2 | 2 | `EOBJ$W_SIZE` | record size |
| 4 | 2 | `EMH$W_HDRTYP` | 0 (`EMH$C_MHD`) |
| 6 | 1 | `EMH$B_STRLVL` | 2 (`EOBJ$C_STRLVL`) |
| 7 | 1 | `EMH$B_TEMP` | 0 |
| 8 | 4 | `EMH$L_ARCH1` | **183** |
| 12 | 4 | `EMH$L_ARCH2` | 0 |
| 16 | 4 | `EMH$L_RECSIZ` | longest record in the module |
| 20 | var | module name | ASCIC, 1–31 characters |
| | var | module version | ASCIC, may be empty |
| | 17 | creation time | `dd-mmm-yyyy hh:mm` |

**Change:** Alpha leaves `EMH$L_ARCH1` unused (must be zero). vaxpunk puts the
architecture code there: 183 for ARM64, the number ELF uses for AArch64
(`EM_AARCH64`). A reader rejects a module whose `ARCH1` is not 183.

## Global symbol directory (`EOBJ$C_EGSD`)

Unchanged, including the quadword alignment of every subrecord. vasm produces:

- `EGSD$C_PSC` (0), program section definitions. Flags, alignment (0–16, a power
  of two, so up to 64 KB) and naming are the Alpha ones, and so are the
  standard psect names (`$CODE$`, `$DATA$`, `$LITERAL$`, `$BSS$`, …).
- `EGSD$C_SYM` (1), symbol definitions (`ESDF$`) and references (`ESRF$`).
  `ESDF$` holds 8-byte value and code address fields, as on Alpha. Procedure
  definitions (`EGSY$V_NORM`) point at a procedure descriptor and an entry
  point. The descriptor's contents belong to the calling standard and are
  opaque here.

`EGSD$C_IDC`, `EGSD$C_PSC64` and the linker-only subrecords (`SPSC`, `SYMV`,
`SYMM`, `SYMG`, `SPSC64`) keep their numbers and layouts but are not produced
yet.

**Change:** every symbol value is 64 bits. On Alpha a value that needed more
than 32 bits set `EGSY$V_QUAD_VAL`. Here the full 8-byte field is always
meaningful, and the flag is set when the value doesn't fit in 32 bits, so the
Alpha rules still hold.

## Text, information and relocation (`EOBJ$C_ETIR`)

A TIR record holds commands. Each has a 2-byte type, a 2-byte size that counts
the whole command, and arguments. The linker runs them on a stack of 64-bit
values, storing bytes at the image location counter.

**Change: 64-bit arithmetic.** Alpha objects compute in 32 bits unless the
module declares the 64-bit structure level. vaxpunk always uses 64 bits, because
every address is 64 bits:

- `STA_GBL` pushes the symbol's full 64-bit value.
- `STA_PQ` and `STO_OFF` add the full 64-bit offset to the psect base. A
  `STA_PQ` value may also feed operators; Alpha requires `STO_OFF` right after.
- Operators work on signed 64-bit values.
- `STO_LW`, `STO_W` and `STO_B` check that the popped value fits the field,
  signed or unsigned, and fail the link otherwise. Storing an address in a
  longword therefore works only when the image lives below 2 GB.

### Kept

| Value | Name | Arguments | Action |
| --- | --- | --- | --- |
| 0 | `STA_GBL` | ASCIC symbol | push symbol value |
| 1 | `STA_LW` | longword | push, sign-extended |
| 2 | `STA_QW` | quadword | push |
| 3 | `STA_PQ` | longword psect index, quadword offset | push psect base + offset |
| 50 | `STO_B` | | pop, store byte |
| 51 | `STO_W` | | pop, store word |
| 52 | `STO_LW` | | pop, store longword |
| 53 | `STO_QW` | | pop, store quadword |
| 54 | `STO_IMMR` | longword count, bytes | pop a repeat count, store the bytes that many times |
| 55 | `STO_GBL` | ASCIC symbol | store symbol value as a quadword |
| 56 | `STO_CA` | ASCIC procedure | store the procedure's entry point as a quadword |
| 59 | `STO_OFF` | | pop (from `STA_PQ`), store as a quadword address |
| 61 | `STO_IMM` | longword count, bytes | store the bytes |
| 100–109, 111, 113, 114 | `OPR_*` | | NOP, ADD, SUB, MUL, DIV, AND, IOR, EOR, NEG, COM, ASH, ROT, SEL |
| 150 | `CTL_SETRB` | | pop, set the location counter |
| 151 | `CTL_AUGRB` | longword | add to the location counter |
| 152–154 | `CTL_DFLOC`, `CTL_STLOC`, `CTL_STKDL` | | debug records only, as on Alpha |

Commands the Alpha manual marks "not supported in structure level 2" stay
unsupported: `STA_LI`, `STA_MOD`, `STA_CKARG`, `OPR_INSV`, `OPR_USH`,
`OPR_REDEF` and `OPR_DFLIT`.

### Dropped

These patch Alpha instructions or manage Alpha linkage. They are invalid in an
ARM64 object:

| Value | Name |
| --- | --- |
| 57, 58 | `STO_RB`, `STO_AB` (Alpha branch displacements) |
| 63 | `STO_LP_PSB` (linkage pair with signature) |
| 64, 65 | `STO_BR_GBL`, `STO_BR_PS` (Alpha 21-bit branch) |
| 200–214 | `STC_*` (conditional linkage and instruction replacement) |

Alpha code reaches data and routines through a linkage section. Whether ARM64
keeps linkage sections is a calling-standard decision. If it does, linkage
pairs come back as new commands, not as the Alpha ones.

### Added: ARM64 instruction stores

Each command writes one instruction at the location counter and advances it
by 4. Its argument is the instruction as a longword, with the relocated field
zero. It pops the target value S. P is the address the instruction is stored
at. The linker computes the field, checks the range and alignment, inserts the
field, and writes the result. An out-of-range or misaligned value is a link
error naming the module and location, never a silent truncation.

| Value | Name | Instructions | Field | Value stored | Check |
| --- | --- | --- | --- | --- | --- |
| 80 | `STO_A64_JUMP26` | `B`, `BL` | imm26, bits 0–25 | (S − P) / 4 | ±128 MB, 4-aligned |
| 81 | `STO_A64_BRANCH19` | `B.cond`, `CBZ`, `CBNZ`, `LDR` (literal), `LDRSW` (literal), `PRFM` (literal) | imm19, bits 5–23 | (S − P) / 4 | ±1 MB, 4-aligned |
| 82 | `STO_A64_BRANCH14` | `TBZ`, `TBNZ` | imm14, bits 5–18 | (S − P) / 4 | ±32 KB, 4-aligned |
| 83 | `STO_A64_ADR` | `ADR` | immlo bits 29–30, immhi bits 5–23 | S − P | ±1 MB |
| 84 | `STO_A64_ADRP` | `ADRP` | immlo bits 29–30, immhi bits 5–23 | (Page(S) − Page(P)) / 4096 | ±4 GB |
| 85 | `STO_A64_ADD_LO12` | `ADD`, `ADDS` (immediate) | imm12, bits 10–21 | S mod 4096 | none |
| 86 | `STO_A64_LDST8_LO12` | byte `LDR`/`STR` (unsigned offset) | imm12, bits 10–21 | S mod 4096 | none |
| 87 | `STO_A64_LDST16_LO12` | halfword loads and stores | imm12 | (S mod 4096) / 2 | S 2-aligned |
| 88 | `STO_A64_LDST32_LO12` | word loads and stores | imm12 | (S mod 4096) / 4 | S 4-aligned |
| 89 | `STO_A64_LDST64_LO12` | doubleword loads and stores | imm12 | (S mod 4096) / 8 | S 8-aligned |
| 90 | `STO_A64_LDST128_LO12` | 128-bit (`Q` register) loads and stores | imm12 | (S mod 4096) / 16 | S 16-aligned |
| 91 | `STO_A64_MOVW_G0` | `MOVZ`, `MOVK` | imm16, bits 5–20 | bits 0–15 of S | S < 2^16 |
| 92 | `STO_A64_MOVW_G0_NC` | | | bits 0–15 of S | none |
| 93 | `STO_A64_MOVW_G1` | | | bits 16–31 of S | S < 2^32 |
| 94 | `STO_A64_MOVW_G1_NC` | | | bits 16–31 of S | none |
| 95 | `STO_A64_MOVW_G2` | | | bits 32–47 of S | S < 2^48 |
| 96 | `STO_A64_MOVW_G2_NC` | | | bits 32–47 of S | none |
| 97 | `STO_A64_MOVW_G3` | | | bits 48–63 of S | none |

Page(x) is x with its low 12 bits cleared. The `ADRP` page is always 4 KB,
whatever the OS page size. In a `MOVZ`/`MOVK` sequence, the checked variant goes
on the highest chunk and `_NC` on the others, as in ELF's `MOVW_UABS` family.

This list covers every instruction-field relocation in Arm's ELF ABI for
AArch64 (AAELF64) that a static, non-TLS image needs. Data relocations need no
new commands: `STA_*`, the `OPR_*` operators and the existing stores express
absolute and PC-relative data (for example `.LONG target - .` is
`STA_PQ`, `STA_PQ`, `OPR_SUB`, `STO_LW`).

Codes 80–97 come from the Alpha "reserved" store range (66–99). Why the design
differs from Alpha's `STO_BR_*` commands: those name the instruction's location
with a psect index and offset. These use the location counter, like every other
store, so an assembler emits code and its relocations as one stream.

## End of module (`EOBJ$C_EEOM`)

Unchanged:

| Offset | Size | Field |
| --- | --- | --- |
| 0 | 2 | `EOBJ$W_RECTYP`, 9 |
| 2 | 2 | `EOBJ$W_SIZE`, 10, or 24 with a transfer address |
| 4 | 4 | `EEOM$L_TOTAL_LPS`, 0 (no linkage pairs) |
| 8 | 2 | `EEOM$W_COMCOD`, 0 success, 1 warning, 2 error, 3 abort |
| 10 | 1 | `EEOM$B_TFRFLG`, bit 0 weak transfer address |
| 11 | 1 | `EEOM$B_TEMP`, 0 |
| 12 | 4 | `EEOM$L_PSINDX`, psect holding the transfer address |
| 16 | 8 | `EEOM$L_TFRADR`, offset of the transfer address in that psect; the Alpha manual shows a longword plus a zero longword, vaxpunk uses all 64 bits |

**Change (provisional):** on Alpha the transfer address names the main routine's
procedure descriptor. Until the calling standard defines ARM64 descriptors, it
names the first instruction of the entry point.
