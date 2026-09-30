# vasm: the assembler

`vasm` assembles one source file into one object module (`docs/object-format.md`).
Instructions are standard ARM64. Directives and macros follow MACRO-64 and
MACRO-32.

```
vasm [/OBJECT=file | -o file] [/INCLUDE=dir | -I dir]... [/NOWARNINGS=NOTPIC | --nowarnings NOTPIC] SOURCE
```

The object file defaults to the source name with `.obj`. Errors and warnings
show file, line and column, the line with a caret, and the macro calls it came
from. Nothing is written if there are errors.

`vmacro`, the MACRO-32 compiler, is vasm with a dialect that translates VAX
instructions; `docs/macro32.md` describes what changes there.

Status: work order step 8. Not there yet: listings, procedure descriptors,
literal pools (`ldr x0, =value`), and floating-point and SIMD arithmetic. Loads
and stores of B/H/S/D/Q registers work.

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
| `name = expr` | assignment, known only in this module; can be assigned again |
| `name == expr` | global assignment |
| `.GLOBAL a, b` | makes symbols global; a global the module doesn't define is a reference |
| `.EXTERNAL a, b` | declares references to other modules |
| `.WEAK a` | weak definition, or weak reference if undefined here |

Using a symbol that is neither defined nor declared is an error. A label can't
be defined twice. An assignment can: each line sees the value assigned last
before it, and a line before any assignment sees the last one in the module.

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

vasm folds only what doesn't change when the image moves: the difference of
two labels in one psect, constants, and branches, `adr` and literal loads to
the same psect. Every other address reaches the object as an address, so that
the linker knows to relocate it (`docs/linker.md`). A label in an `ABS` psect
is a constant to the linker.

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
| `.SAVE_PSECT [LOCAL_BLOCK]` | remembers the psect, and the local label block if asked |
| `.RESTORE_PSECT` | goes back to the psect saved last |
| `.BYTE`, `.WORD`, `.LONG`, `.QUAD` | 1, 2, 4 and 8-byte values; addresses become relocations |
| `.ADDRESS` | a 64-bit address (as in MACRO-64) |
| `.ASCII`, `.ASCIZ`, `.ASCIC` | text; with a zero byte after; with a length byte before |
| `.ASCID` | a VMS static text descriptor (length, type 14, class 1, 32-bit pointer), then the text |
| `.ASCID64` | the 64-bit form (`DSC64$`: 1, type 14, class 1, -1, 64-bit length, 64-bit pointer), then the text |
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

### Position independence

A `PIC` psect claims that it can be shared unchanged wherever the image goes,
so it mustn't hold an address the loader would have to fix up
(`docs/linker.md`). The linker checks every module; vasm warns early about
what it can see:

```
hello.mar:12:18: warning: %VASM-W-NOTPIC, address needing a fixup in PIC psect $CODE$
```

- `.ADDRESS`, `.QUAD` and `.LONG` of an address in this module;
- `movz` or `movk` with `:abs_g1:` or above of an address in this module;
- `.ASCID` and `.ASCID64`, whose descriptors hold the address of their text.
  Write descriptors in `$DATA$` or `$LINK$`. `.ASCID`'s pointer is a longword,
  which ties the image to the low 2 GB; `.ASCID64`'s is a quadword.

An external symbol may be a constant, so it waits for the linker.
`/NOWARNINGS=NOTPIC` turns the warnings off.

## Macros

As in MACRO: a macro is a block of text whose formal arguments are replaced by
the text of the call's arguments.

```
        .MACRO  CHECK   REG, VALUE=0, ?OK       ; formals: plain, default, created label
        cmp     REG, #VALUE
        b.eq    OK
        $EXIT   STATUS=^X2C
OK:
        .ENDM   CHECK

        CHECK   x0, 11                          ; positional
        CHECK   VALUE=3, REG=x1                 ; keyword
```

- Arguments are separated by commas. `<...>` groups one argument that holds
  commas, and loses its brackets. vasm also keeps `[...]`, `(...)` and
  `"..."` together, so `[x1, #8]` is one argument.
- A missing or empty argument takes its default, or else is empty.
- `?NAME` creates a local label, `30000$`, `30001$`... for each call, unless
  the call gives one. Created labels are unique in the module, so a macro can
  define one in another psect and use it here.
- Formals are replaced wherever they appear as a whole name, strings included.
  An apostrophe next to a formal concatenates and disappears: `NAME'_COUNT`.
  (So `'A'` is not a character constant inside a macro with a formal `A`.)
- Macros can call macros, and themselves, up to 100 levels. A macro's name
  can shadow an instruction.
- `.NARG sym` sets `sym` to the number of positional arguments given, empty
  ones included. `.MEXIT` leaves the innermost macro.

### Conditionals

```
        .IF     condition argument
        ...                                     ; condition true
        .IF_FALSE                               ; or .IFF, or .ELSE
        ...                                     ; condition false
        .IF_TRUE_FALSE                          ; or .IFTF: either way
        ...
        .ENDC
        .IIF    condition argument, statement
```

`.IF_TRUE` (`.IFT`) switches back to the true part. The comma after the
condition is optional. Conditions:

| Condition | True when |
| --- | --- |
| `EQ`, `NE`, `GT`, `LT`, `GE`, `LE` (or `EQUAL`, `NOT_EQUAL`, `GREATER`, `LESS_THAN`, `GREATER_EQUAL`, `LESS_EQUAL`) | the constant expression compares so with 0 |
| `DF`, `NDF` (`DEFINED`, `NOT_DEFINED`) | the symbol is (not) defined at this point |
| `B`, `NB` (`BLANK`, `NOT_BLANK`) | the argument, usually `<ARG>`, is (not) blank |
| `IDN`, `DIF` (`IDENTICAL`, `DIFFERENT`) | the two arguments are the same text, ignoring case, or not |

A conditional opened in a macro must end in it.

### Repeat blocks

| Block | Repeats its body |
| --- | --- |
| `.REPEAT n` (`.REPT`) ... `.ENDR` | `n` times |
| `.IRP sym, <a, b, c>` ... `.ENDR` | once per argument, with `sym` replaced by it |
| `.IRPC sym, <text>` ... `.ENDR` | once per character |

### Macro libraries

`.LIBRARY "file"` reads macro definitions from a file, looked up next to the
source, then in each `-I` directory. A library holds only `.MACRO` blocks, and
comments. Unlike VMS `.MLB` files, it is text. A macro defined in the source
wins over a library's.

`vtools/lib/vrun.mlb` wraps vrun's monitor calls (`docs/runner-abi.md`):
`$PUT "text"<10>`, `$WRITE address, length`, `$EXIT [STATUS=value]` and
`$DUMP`. `$PUT` stores its text in `$LITERAL$`.

`vtools/lib/pic.mlb` is for position-independent code: `$LDADR reg, symbol`
loads `symbol`'s address into `reg` from a linkage slot of its own in `$LINK$`,
which the loader fixes up if the image moves.

## Output

The module header gets the module name, the `.IDENT` version, and the creation
time in UTC. `SOURCE_DATE_EPOCH`, if set, replaces the current time, for
reproducible builds. The GSD has every psect, global definitions and external
references. A global label in a `REL` psect is an address, with `EGSY$V_REL`;
absolute global symbols go into `$ABS$`, and labels in `ABS` psects are
constants too. Each psect's contents
follow as TIR commands. Uninitialized space (`.BLKx`, `.ALIGN` padding) is
skipped with `CTL_AUGRB`, and the linker fills it with zeros.
