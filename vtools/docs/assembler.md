# vasm: the assembler

`vasm` assembles one source file into one object module (`docs/object-format.md`).
Instructions are standard ARM64. Directives follow MACRO-64 and MACRO-32.

```
vasm [/OBJECT=file | -o file] SOURCE
```

The object file defaults to the source name with `.obj`. Errors show file, line
and column, with the line and a caret, and nothing is written.

Status: work order step 5. Not there yet: macros (`.MACRO`, `.IF`, `.IRP`,
`.LIBRARY`), listings, procedure descriptors, literal pools (`ldr x0, =value`),
and floating-point and SIMD arithmetic. Loads and stores of B/H/S/D/Q registers
work.

## Lines

```
label:  mnemonic operands       ; comment
```

A line holds any number of labels, then at most one instruction, directive or
assignment. Comments start with `;` (MACRO) or `//` (GNU), except inside quotes.

Names are letters, digits, `$`, `_` and `.`, not starting with a digit. They
are case-insensitive: everything is folded to upper case, so the object file
has `HELLO` wherever the source has `hello`. Mnemonics and register names are
case-insensitive too.

## Labels and symbols

| Form | Meaning |
| --- | --- |
| `name:` | label, known only in this module |
| `name::` | global label, visible to the linker |
| `10$:` | local label: valid between two ordinary labels, or until the psect changes |
| `name = expr` | assignment, known only in this module |
| `name == expr` | global assignment |
| `.GLOBAL a, b` | makes symbols global; a global the module doesn't define is a reference |
| `.EXTERNAL a, b` | declares references to other modules |
| `.WEAK a` | weak definition, or weak reference if undefined here |

Using a symbol that is neither defined nor declared is an error.

## Expressions

C operators and precedence: unary `-`, `~`, `+`, then `*` `/` `%`, then `+` `-`,
`<<` `>>` (logical), `&`, `^`, `|`. Parentheses group.

Numbers are decimal, `0x1F`, `0b101` or `0o17`, or with VMS radix prefixes
`^X1F`, `^B101`, `^O17`, `^D10`. `'A'` is a character's code. `.` is the
current location.

A value is a constant, an address in one of this module's psects, or an
external symbol. An address plus or minus a constant is still an address, and
the linker resolves it. Two addresses in the same psect subtract to a constant,
so `length = . - start` works. Other arithmetic on addresses is an error.

## Instructions

Standard ARM64 syntax, as GNU `as` and LLVM accept it, including aliases such as
`mov`, `cmp`, `lsl`, `cset`, `ubfx`, `sxtw`, `mul` and `ret`. `#` before an
immediate is optional. `tests/encode.rs` lists every supported form and checks
each one bit for bit against GNU `as`.

Supported: integer arithmetic and logic (register, shifted, extended and
immediate forms, logical bitmask immediates), move wide, bit fields, multiply
and divide, conditional select, bit and byte reversal, branches (`b`, `bl`,
`b.cond`, `cbz`, `tbz`, `br`, `blr`, `ret`), `adr` and `adrp`, loads and
stores (unsigned offset, unscaled, pre- and post-index, register offset,
literal, pairs, exclusive and ordered), `svc`, `hvc`, `brk`, `hlt`, `udf`,
hints, barriers, `mrs` and `msr` (common registers by name, any as
`s3_3_c13_c0_2`), and `eret`.

A branch, `adr` or literal load whose target is in the same psect is encoded
directly. Anything else becomes a relocation for the linker: targets in other
psects, external symbols, and every `adrp`. Relocation operators work as in GNU
syntax:

| Operator | Use |
| --- | --- |
| `adrp x0, sym` | 4 KB page of `sym` |
| `add x0, x0, #:lo12:sym` | low 12 bits of `sym` |
| `ldr x1, [x0, #:lo12:sym]` | low 12 bits, scaled by the access size; `sym` must be aligned to it |
| `movz x0, #:abs_g1:sym` | bits 16-31 of `sym`, which must fit in 32 bits (`_nc` variants: no check) |

## Directives

| Directive | Effect |
| --- | --- |
| `.TITLE name text` | module name (default: the file name) |
| `.IDENT /V1.0/` | module version |
| `.PSECT name, attributes` | switches to a psect, creating it on first use |
| `.BYTE`, `.WORD`, `.LONG`, `.QUAD` | 1, 2, 4 and 8-byte values; addresses become relocations |
| `.ADDRESS` | a 64-bit address (as in MACRO-64) |
| `.ASCII`, `.ASCIZ`, `.ASCIC` | text; with a zero byte after; with a length byte before |
| `.ASCID` | a VMS static text descriptor (length, type 14, class 1, 32-bit pointer), then the text |
| `.BLKB n`, `.BLKW`, `.BLKL`, `.BLKQ` | `n` zero bytes, words, longwords or quadwords (default 1) |
| `.ALIGN n` | aligns to 2^`n`, or `BYTE`, `WORD`, `LONG`, `QUAD`, `OCTA`, `PAGE` (64 KB) |
| `.END label` | ends the source; the label is the transfer address |

Text is written as in MACRO: between two copies of any delimiter (`/text/`,
`"text"`), with `<expr>` for single bytes, all in sequence:
`.ASCIZ "Hello"<13><10>`. A `/`-delimited string can't contain `;` or `//`,
which start comments.

### Psects

`.PSECT name` alone switches to the psect, or creates it with its default
attributes. The standard names get the attributes the Alpha compilers use:

| Psect | Attributes |
| --- | --- |
| `$CODE$` | PIC CON REL LCL SHR EXE NORD NOWRT |
| `$DATA$` | NOPIC CON REL LCL NOSHR NOEXE RD WRT |
| `$LINK$` | NOPIC CON REL LCL NOSHR NOEXE RD NOWRT |
| `$LITERAL$`, `$READONLY$` | PIC CON REL LCL SHR NOEXE RD NOWRT |
| `$BSS$` | NOPIC CON REL LCL NOSHR NOEXE RD WRT NOMOD |
| any other | NOPIC CON REL LCL NOSHR NOEXE RD WRT |

All start QUAD aligned. Code before any `.PSECT` goes into `$CODE$`.

Attributes listed after the name start from the last row and change it:
`PIC`/`NOPIC`, `CON`/`OVR`, `REL`/`ABS`, `LCL`/`GBL`, `SHR`/`NOSHR`,
`EXE`/`NOEXE`, `RD`/`NORD`, `WRT`/`NOWRT`, `VEC`/`NOVEC`, `NOMOD`, and an
alignment keyword or number. A psect can't be both `EXE` and `WRT`. Naming a
psect again with different attributes is an error.

## Output

The module header gets the module name, the `.IDENT` version, and the creation
time in UTC. `SOURCE_DATE_EPOCH`, if set, replaces the current time, for
reproducible builds. The GSD has every psect, global definitions and external
references. Absolute global symbols go into `$ABS$`. Each psect's contents
follow as TIR commands. Uninitialized space (`.BLKx`, `.ALIGN` padding) is
skipped with `CTL_AUGRB`, and the linker fills it with zeros.
