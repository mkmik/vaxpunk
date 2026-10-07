# vbliss: the BLISS-64 compiler

`vbliss` compiles BLISS-64 source for ARM64 into an object module
(`docs/object-format.md`), which `vlink` links like any other. It is stage 0
of [PRD-0004](../../docs/prd/0004-bliss64-compiler.md): the compiler in Rust
that will compile BLISS.EXE, the compiler in BLISS-64. `docs/bliss64.md`
describes the dialect.

```
vbliss [/OBJECT=file | -o file] [/LIST[=file]] [/VARIANT=n] [--ir] [--asm] SOURCE
```

The object file defaults to the source name with `.obj`. `--ir` prints the
IR instead, `--asm` the vasm source. `/LIST` writes the listing, by default
to the source name with `.lis`; `/VARIANT` sets `%VARIANT`, 1 if no value
is given. `REQUIRE 'NAME'` reads `NAME`, or `NAME.R64` or `NAME.REQ`, in
either case, from the source's directory. Diagnostics go to stderr as
BLISS writes them on a terminal: the line, a marker under the place, the
message and where.

```
vbliss hello.b64
vmacro -o putoutput.obj vtools/lib/putoutput.mar
vmacro -o conputchar.obj vtools/lib/conputchar.mar
vlink hello.obj putoutput.obj conputchar.obj && vrun hello.exe
```

Status (PRD-0004 step 4): modules with `MAIN` and `IDENT`; `ROUTINE`,
`GLOBAL ROUTINE`, `EXTERNAL ROUTINE`, `FORWARD ROUTINE`, `NOVALUE`; `OWN`,
`GLOBAL`, `EXTERNAL`, `LOCAL` and `STACKLOCAL` data with an allocation unit,
`SIGNED` or `UNSIGNED` and `INITIAL`; `LITERAL`; `LABEL`; every operator;
field references `<p, s, e>`; `IF`, `CASE`, `SELECT` and `SELECTONE` with
their `U` and `A` forms, `INCR` and `DECR` likewise, `WHILE`, `UNTIL`, `DO`,
`LEAVE`, `EXITLOOP`, `RETURN`; calls by the calling standard, through a
routine's name or any address; `%ASCID`, `%ASCII`, `%C`, `%X`, `%O`, `%B`,
`%DECIMAL`. Step 5: `MACRO` and `KEYWORDMACRO` with simple, conditional
and iterative macros, `COMPILETIME`, `REQUIRE` and `%REQUIRE`, the
lexical conditionals and the lexical functions, and the listing. Step 6:
`STRUCTURE`, the predeclared `VECTOR`, `BITVECTOR`, `BLOCK`,
`BLOCKVECTOR` and `BLOCK_BYTE`, `REF`, `FIELD` and field sets, ordinary
and general structure references, `BIND`, `BIND ROUTINE`, `MAP`, `PLIT`
and `UPLIT`, `INITIAL` and `PRESET` on static and `LOCAL` data, and
structure attributes on formals. Not yet: `GLOBAL BIND`, default
structure references, `PSECT`, linkages, built-ins, conditions.

## How it works

Five passes, each a module of the crate, which BLISS.EXE will repeat:

1. `lex.rs` reads lexemes: names in upper case, decimal numbers, quoted
   strings, special characters and `%`; comments go.
2. `lexical.rs`, the lexical processor, expands macro calls, lexical
   functions and lexical conditionals, puts `%X'1F'` and the like
   together, and reads require files, as LRM chapters 15 and 16 describe.
3. `parse.rs` parses declarations and expressions, and resolves each name
   when it is used, since BLISS declares everything first. A literal
   becomes its value.
4. `irgen.rs` turns each routine into the IR and the data into items.
5. `arm64.rs` turns the IR into vasm source, which vasm assembles.

The parser pulls one lexeme at a time from the lexical processor, as BLISS
reads a module, so a macro is known from the lexeme after its
declaration. Lexemes come from a stack of streams: the source file at the
bottom, require files, and each expansion on top until it has been read. A
macro body is kept as lexemes and formals (macro-quote level, with the
quote functions done); a call's actuals are read expanded (name-quote
level), put into a copy of the body, and the copy is read again. Only
macro names are bound there: the parser binds the rest. A lexical
function's expression parameter is parsed by the parser on its own
lexemes. `%REMAINING`, `%LENGTH` and `%COUNT` belong to the copy they are
read from, and an iterative macro's copies are a stream each, so
`%EXITITERATION` leaves its separator behind.

## Definitions

`vtools/lib/lib.r64` and `starlet.r64`, and `lib.req` and `starlet.req`
for BLISS-32, are what `vdefs` (`vtools/crates/vdefs`, `just defs`)
makes of the `$xxxDEF` macros in `lib.mlb` and `starlet.mlb`, so BLISS
and MACRO-32 code read one definition of each structure. A symbol's kind
comes from the letters between its `$` and `_`, as VMS names encode it:

| Letters | Becomes |
| --- | --- |
| `B`, `W`, `L`, `Q` | a field macro, `PCB$L_STS = 20, 0, 32, 0 %`: offset, position, size, extension, for a `BLOCK[, BYTE]` reference |
| `A`, `IS`, `PS` | a 32-bit field, sign-extended |
| `IH`, `PH`, `PQ` | a 64-bit field |
| `T`, `AB`, `AW`, `AL`, `AQ` | a field of size 0: its address |
| `K`, `C`, `M`, `S`, `V`, none | a `LITERAL` |

Under BLISS-32 a 64-bit field has size 0, as in DEC's `STARLET.REQ`. A
`$V_` name is the bit number, not DEC's field macro: the macro libraries
don't say which field the bit is in, so BLISS code writes
`.PCB[PCB$L_STS]<PCB$V_WALL, 1>`. Letters the rule doesn't know stop
vdefs with the symbol's name, to be fixed in the macro library. vdefs's
test fails when the files are stale. `LIBRARY 'SYS$LIBRARY:LIB'` reads
`lib.r64` from the include path (`-I vtools/lib`), and the vrun tests pass
that path; `tests/bliss/defs` reads a PCB that MACRO-32 filled through
`$PCBDEF`.

## Structures

A structure's size and body are parsed once, when it is declared, with
its formals as names of their own (`data.rs`); the predeclared ones are
declared the same way from BLISS-64 text at the start of each module. A
structure reference is a copy of the body with the formals replaced: the
segment's address (fetched for `REF`), the allocation actuals, which are
constants, and the access actuals, each evaluated once into an IR
temporary unless it is a constant or a name. What remains is an ordinary
expression, usually a field reference, which `irgen` reads and writes
with the smallest load that holds the field. `PLIT` items and `INITIAL`
values are packed with no alignment, a `PLIT`'s count is its bytes in
fullwords rounded up, and a string fills out its last unit with zeros,
as BLISSA64 lays them out. A run-time `BIND` and a `LOCAL`'s `INITIAL` or
`PRESET` are assignments at the start of their block.

## The listing

`listing.rs` writes the source part of the listing as BLISSA64 does with
`/SOURCE_LIST=(EXPAND_MACROS,REQUIRE)`, so the listings of `tests/bliss`
compare with the oracle's line for line once page headers and the closing
summary are dropped (the `listing` test):

- A source line is listed when the parser asks for a lexeme on a later
  line, with the block depth at that moment, so a block's last line still
  shows its depth; a block closes in the listing once the lexeme after it
  has been read. `BEGIN` blocks and parenthesized ones both count.
- The flag column holds `R` for a require file's lines, and `P` or `L`
  for lines read inside a macro call's or a lexical function's actuals.
  Lines are cut at 116 characters.
- After a line come its diagnostics, with a marker line under it, then
  what happened while it was the last line read: `%PRINT`'s text and each
  expansion, `[NAME]=` and the lexemes it gave once read, `null` if
  there were none to read. An expansion is indented by the macro calls in
  progress when it began, and wraps by column 84.
- Diagnostics mark the lexeme they are about, or, if a macro body gave it,
  the last one read from the source.

A name is its address: `.X` fetches X's own field, its allocation unit and
extension, or a fullword at the address another expression gives. A field
`<p, s>` of constant position and size reads the smallest of 1, 2, 4 and 8
bytes that holds it, then `ubfx` or `sbfx`; a store reads, inserts with
`bfi` and writes back. Tests take the low bit (`tbnz`), as BLISS does.
Constant expressions are folded, and a test of a constant branches
straight to its side. That is all the optimization there is.

## The IR

The contract between vbliss and BLISS.EXE: both print the same IR for the
same source, and `tests/bliss/NAME.ir` holds it for some programs. Each
routine is a list of blocks `@n`, of three-address instructions on
temporaries `%n`, as QBE's IR but without SSA; every value is a 64-bit
fullword (`=l`). Operands are temporaries, constants, a symbol's address
`$NAME+8`, or a frame slot's `&n`.

```
global routine $FIB {
    &0 = slot 8
@0
    %0 =l arg 0
    store8 %0, &0
    %2 =l load8u &0
    %3 =l clt %2, 2
    jlbs %3, @1, @2
```

| Instruction | Does |
| --- | --- |
| `add sub mul div rem and or xor eqv` | arithmetic and logic; `div` truncates, `rem` has the dividend's sign |
| `shl shr sar` | shifts by 0-63: left, logical right, arithmetic right |
| `ash` | BLISS's `^`: left by a positive count, arithmetic right by a negative one |
| `ceq cne clt cle cgt cge`, `cltu cleu cgtu cgeu` | comparisons, 1 or 0, signed or unsigned |
| `copy neg not` | |
| `loadNs`, `loadNu` | N bytes, sign- or zero-extended |
| `storeN value, addr` | |
| `exts`, `extu value, pos, size` | a bit field of a value |
| `ins base, value, pos, size` | base with a bit field replaced |
| `arg n` | the routine's argument n |
| `call target(args)` | by the calling standard |
| `jmp @n`, `jlbs v, @t, @f`, `ret v` | ends a block |

Data prints as `data $NAME in PSECT align A { items }`, each item a size and
a value (`4 $P.1`), a string, or `z N` zero bytes.

## Code

Routines follow the calling standard (`docs/design/0004-calling-standard.md`):
arguments in x0-x7 and on the stack, x9 set to their count, the result in
x0. Every routine has a frame and a frame descriptor in `$CODE$_FDSC`, laid
out as `call.mlb`'s `$ROUTINE` lays them out: the frame record, the handler
at 16(FP), the descriptor's address at 24(FP), the registers it keeps from
32(FP), then the IR's slots and the spills.

Temporaries live in x19-x28, which survive calls, chosen by a linear scan
over their live ranges; those that don't fit go to the frame. x10-x17 are
scratch. Data goes in BLISS's psects: `$OWN$`, `$GLOBAL$`, and `$PLIT$` for
descriptors and their text; code in `$CODE$`.

## Tests

`tests/bliss` holds BLISS-64 programs that print with `PRINT`
(`tests/bliss/print.r64`) through `LIB$PUT_OUTPUT`, which `lib/putoutput.mar`
provides under vrun. vrun's `programs` test compiles, links and runs each
and compares what it prints with `NAME.stdout`, which is what the same
program printed on OpenVMS Alpha compiled by DEC's BLISSA64: the oracle,
`ods/vms/bliss-oracle.py`, writes it, with the listing in `NAME.lis` and
the compiler's messages in `NAME.oracle`.
