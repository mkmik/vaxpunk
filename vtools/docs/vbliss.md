# vbliss: the BLISS-64 compiler

`vbliss` compiles BLISS-64 source for ARM64 into an object module
(`docs/object-format.md`), which `vlink` links like any other. It is stage 0
of [PRD-0004](../../docs/prd/0004-bliss64-compiler.md): the compiler in Rust
that will compile BLISS.EXE, the compiler in BLISS-64. `docs/bliss64.md`
describes the dialect.

```
vbliss [/OBJECT=file | -o file] [--ir] [--asm] SOURCE
```

The object file defaults to the source name with `.obj`. `--ir` prints the
IR instead, `--asm` the vasm source. `REQUIRE 'NAME'` reads `NAME`, or
`NAME.R64` or `NAME.REQ`, in either case, from the source's directory.

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
`%DECIMAL`. Ahead of their steps: `VECTOR[n, unit, ext]` and plain
`REQUIRE`. Not yet: macros and lexical functions, the other structures,
`BIND`, `PLIT`, linkages, built-ins, conditions, listings.

## How it works

Four passes, each a module of the crate, which BLISS.EXE will repeat:

1. `lex.rs` reads lexemes: names in upper case, numbers with their radix
   applied, quoted strings, special characters; comments go.
2. `parse.rs` parses declarations and expressions, and resolves each name
   when it is used, since BLISS declares everything first. A literal
   becomes its value; a `REQUIRE` splices in the file's lexemes.
3. `irgen.rs` turns each routine into the IR and the data into items.
4. `arm64.rs` turns the IR into vasm source, which vasm assembles.

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
