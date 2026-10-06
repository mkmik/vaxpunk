# PRD-0004 — A BLISS-64 compiler for ARM64, bootstrapped in Rust and then written in BLISS-64

Oct 6, 2026 · @Marko Mikulicic

## Context and goal

Everything above the PAL is meant to be BLISS and MACRO
([ADR-0001](../adr/0001-pal-interface-vms-vocabulary.md)), and so far it is
only MACRO: about 25,000 lines of MACRO-32 compiled by `vmacro`. MACRO-32
is fine for drivers, the scheduler and anything that depends on IPL. It
gets expensive in long routines that are mostly logic, such as DCL's
procedures and lexical functions, LOGINOUT, AUTHORIZE or a lock manager,
where every routine has to state and keep a register contract.

MACRO-32 is also 32-bit by design. A MACRO-32 pointer stored in memory is
a longword, which is why the address space sits below 2 GB
([ADR-0005](../adr/0005-access-modes-are-threads.md)). 64-bit code, for P2
space, `_64` services and 64-bit userland, needs a language whose fullword
is 64 bits.

BLISS-64 is that language, and it is VMS's own: DEC wrote much of the
executive and most utilities in BLISS, and BLISS-64 was its Alpha dialect.
A BLISS-64 compiler gives vaxpunk a systems language that interoperates
with MACRO-32 routine by routine, through `LINKAGE`, and is 64-bit from
the start.

Goals:

- **BLISS-64, and only BLISS-64.** The dialect DEC's BLISS-64EN compiler
  accepts on OpenVMS Alpha, retargeted to ARM64.
- **Cross-compilation on the Mac.** The compiler runs on the host and writes
  OBJ modules that `vlink` links, as part of the system build.
- **Self-hosting.** The compiler is written in BLISS-64. On the Mac it runs
  under `vrun`. The first BLISS-64 compiler, the one that compiles it, is
  written in Rust.
- **A simple code generator of our own.** A small IR and a direct ARM64
  back end in the spirit of QBE. No LLVM, no optimizer worth the name.
- **`vasm` ported to BLISS-64.** The assembler becomes a BLISS-64 module
  that `BLISS.EXE` links in, so the compiler writes OBJ modules itself,
  under `vrun` and, as an image, on vaxpunk. Compiling and assembling on
  vaxpunk is the first step to a native toolchain.

Decisions this PRD rests on:

| Decision | Choice |
| --- | --- |
| Dialects | BLISS-64 (`/A64`, the default; `%BPVAL` is 64, a fullword is a quadword) and BLISS-32 (`/A32`, as BLISS-32EN on Alpha) so existing 32-bit code recompiles; the compiler itself is BLISS-64 |
| Spec | DEC's BLISS Language Reference Manual (1987), plus a BLISS-64 delta we write from DEC's Alpha documentation and the reference compiler |
| Reference compiler | BLISSA64 V1.11-7 on OpenVMS Alpha in AXPbox: an oracle for listings and behaviour, never for machine code |
| Calling standard | The vaxpunk calling standard ([ADR-0023](../adr/0023-calling-standard.md)): AAPCS64 plus an argument count in a register |
| Output | `vasm` assembly text, assembled by `vasm`: the Rust crate in stage 0, its BLISS-64 port in `BLISS.EXE` |
| Bootstrap | The Rust compiler (stage 0) compiles the BLISS-64 compiler (stage 1), which compiles itself (stage 2) |

## Non-goals

- **No BLISS-16 or BLISS-36.** BLISS-32 is in, as BLISS-32EN was on
  Alpha: VAX BLISS-32 code that uses the VAX's built-ins, `AP` or `FP`
  still needs the edits Alpha needed.
- **No LLVM** and no other compiler framework. The same rule as
  [PRD-0001](0001-vtools.md): LLVM tools may only be test oracles.
- **No optimizations** beyond what keeps the code from being silly:
  constant folding, using registers instead of the frame where it is easy,
  and not generating dead branches for compile-time conditions. No SSA, no
  loop optimizations, no inlining.
- **No native linking yet.** `BLISS.EXE` with the `vasm` port compiles
  and assembles on vaxpunk, but `vlink` and `vlib` stay Rust on the Mac:
  objects made on vaxpunk are linked on the host until they are ported
  too, in a PRD of their own.
- **No `vmacro` port.** MACRO-32 sources still compile only on the Mac.
- **No debug symbol records (DST).** The listing and the link map are the
  debugging aids, as for MACRO-32.
- **No DEC binary formats.** Precompiled libraries (`.L64`) are our own
  format, not DEC's. STARLET and LIB come from vaxpunk's definitions, never
  from the HP kit.
- **No copying.** The HP manuals, the BLISSA64 kit, the BLISS-11 sources on
  the Freeware CD and the VAX/VMS V4.3 sources are read for reference only.
  The repo holds our notes in our own words.

## Component overview

```
  stage 0 (Rust, host)
  foo.b64 ──► vbliss ──► foo.obj ──► vlink ──► foo.exe
               │ └── vasm (as a library, like vmacro)
               └── LIB.R64, STARLET.R64  (vdefs, from lib.mlb and starlet.mlb)

  stage 1+ (BLISS-64, under vrun on the host, or an image on vaxpunk)
  foo.b64 ──► BLISS.EXE ──► foo.obj
               └── VASM, vasm ported to BLISS-64, linked in
```

**`vbliss`** (`vtools/crates/vbliss`) is the stage 0 compiler, in Rust: a
front end (lexer, macro expander, parser, name resolution), the IR, the
ARM64 back end, the listing and the lint. It links `vasm` as a library and
writes an OBJ in one command, as `vmacro` does.

**`BLISS.EXE`** is the BLISS-64 compiler written in BLISS-64
(`vtools/bliss/`). It is a port of `vbliss`, with the same passes, the same
IR and the same output. It reads and writes files through a small I/O
module with two implementations: `vrun`'s file monitor calls on the host,
and RMS on vaxpunk.

**VASM** is `vasm` ported to BLISS-64 (`vtools/bliss/vasm/`): the lexer,
macro facility, directives, ARM64 encoder and OBJ writer, with the same
output as the Rust `vasm` byte for byte. `BLISS.EXE` links it and hands it
the assembly it generates; it also builds alone as `VASM.EXE`, an
assembler for `.MAR` sources of plain ARM64.

**A driver**, `vbliss --stage1`, runs `BLISS.EXE` under `vrun`, so the
build calls one command whichever compiler it uses.

**`vdefs`** turns the `$xxxDEF` macros of `lib.mlb` and `starlet.mlb` into
`LIB.R64` and `STARLET.R64`. The macro libraries stay the single source of
the definitions.

## Language

The language is the BLISS Language Reference Manual (AA-H275E-TK, May
1987), common BLISS and its BLISS-32 parts, as Alpha's BLISS-64 and
BLISS-32 change them. The
1987 manual predates Alpha, so the first deliverable is
`vtools/docs/bliss64.md`: the BLISS-64 differences, each with where it was
learned (DEC's Alpha BLISS documentation, the kit's release notes, or a
probe of the reference compiler). Among them: 64-bit fullwords and
`%BPVAL`, `%UPVAL` and the allocation units, quadword fields, the Alpha
linkages and built-ins, how BLISS-64 treats longword data in 32-bit
VMS structures, the `LONG_DEFAULT`, `REF_LONG` and `SIGNED_LONG` switches
for moving BLISS-32 code to BLISS-64, and what `/A32` does.

What the compiler must accept, by the end of this PRD:

- Modules and their switches; `ROUTINE`, `GLOBAL ROUTINE`,
  `EXTERNAL ROUTINE`, `FORWARD ROUTINE`.
- Data: `OWN`, `GLOBAL`, `EXTERNAL`, `LOCAL`, `STACKLOCAL`, `REGISTER`,
  `GLOBAL REGISTER`, `EXTERNAL REGISTER`, `BIND`, `LITERAL`,
  `GLOBAL LITERAL`, `EXTERNAL LITERAL`, `PSECT`.
- Expressions, all operators and the fetch (`.`) and assignment rules, with
  sign and zero extension as the manual gives them.
- Control: `BEGIN`/`END` blocks with values, `IF`, `CASE`, `SELECT`,
  `SELECTONE` and their `U`/`A` forms, `INCR`, `DECR`, `WHILE`, `UNTIL`,
  `DO`, `LEAVE`, `EXITLOOP`, `RETURN`.
- Structures: `VECTOR`, `BITVECTOR`, `BLOCK`, `BLOCKVECTOR`, user
  `STRUCTURE` declarations, `REF`, `FIELD` and field sets.
- `PLIT`, `UPLIT` and the string forms (`%ASCII`, `%ASCIZ`, `%ASCIC`,
  `%ASCID`).
- The macro language: simple, keyword, iterative and recursive macros,
  `%REMAINING`, `%COUNT`, `%LENGTH`, `%IF`, `%QUOTE`, `%UNQUOTE`,
  `%EXPAND`, and the other lexical functions; `REQUIRE` and `LIBRARY`.
- `LINKAGE`: `CALL` and `JSB` with `REGISTER`, `GLOBAL`, `PRESERVE`,
  `NOPRESERVE` and `NOTUSED`, so BLISS-64 calls MACRO-32 routines with
  their register contracts unchanged and MACRO-32 calls BLISS-64.
- `BUILTIN`, with the built-ins vaxpunk needs, under BLISS-64's names:
  the PAL calls as `PAL_x` (`PAL_MTPR_IPL`, `PAL_PROBER`, `PAL_INSQHIL`)
  and `CALL_PAL`, the atomics, `ACTUALCOUNT`, `ACTUALPARAMETER`,
  `ARGPTR`, each listed in `bliss64.md`. BLISS-64 has no VAX built-ins:
  no `CALLG`, which is `LIB$CALLG`.
- `ENABLE`, `ESTABLISH`, `REVERT` and `SIGNAL`, `SIGNAL_STOP`, `SETUNWIND`, with handlers found
  along the frame chain as for MACRO-32
  ([ADR-0021](../adr/0021-condition-handlers-run-in-the-mode-that-signals.md)).

Names are case-insensitive and folded to upper case, as in `vasm`. Sources
are `.B64`, require files `.R64`, libraries `.L64`; under `/A32`, `.B32`,
`.R32`, `.L32`; `.BLI`, `.REQ` and `.LIB` under both.

## Definitions

BLISS-64 code needs the same structures as MACRO-32 code: `$PCBDEF`,
`$UCBDEF`, `$SSDEF`, and the rest. Two hand-kept copies would drift, and
every drift is a silent wrong-offset bug. `vdefs` reads the `$xxxDEF`
macros in `lib.mlb` and `starlet.mlb` and writes `LIB.R64` and
`STARLET.R64`, and `LIB.REQ` and `STARLET.REQ` for `/A32`, with each symbol's size from its name, as VMS names encode
it: `PCB$L_SQFL` is a longword at offset 0, `$V_` and `$S_` pairs are bit
fields, `$K_` and `$C_` are constants, `$M_` masks. A field the naming
rule can't type is an error that names it, fixed in `lib.mlb`. The build
regenerates the `.R64` and `.REQ` files and CI fails when they are stale. The same
tool can write C headers later.

## Code generation

**IR.** Each routine becomes a list of three-address instructions on
virtual registers, in the spirit of QBE's IR: a textual form that
`vbliss --ir` prints and tests compare, two classes (64-bit `l` and 32-bit
`w`), loads and stores of a size and signedness, calls, and branches
between labelled blocks. No SSA. The IR is the contract between the two
implementations: `vbliss` and `BLISS.EXE` print the same IR for the same
source.

**Back end.** Instruction selection a node at a time, with a few patterns
for addressing modes and field extracts (`ldr` with an offset, `ubfx`,
`sbfx`, `bfi`). Register allocation is a linear scan over each routine's
virtual registers, spilling to the frame; the registers a `LINKAGE` fixes
are pre-colored. Output is `vasm` source, with `.PSECT` for BLISS's default
program sections (`$CODE$`, `$PLIT$`, `$OWN$`, `$GLOBAL$`) and the module's
`PSECT` declarations.

**Calling standard.** The compiler follows
[DESIGN-0004](../design/0004-calling-standard.md)
([ADR-0023](../adr/0023-calling-standard.md)): arguments in registers and
on the stack with their information in x9, 32-bit values sign-extended,
the frame record, the handler at `16(FP)` and a frame descriptor at
`24(FP)` for every routine with a frame. `JSB` linkages call MACRO-32's
declared JSB routines with `bl`, and trust their declarations for what
they keep. `vmacro` moves to the standard
([PRD-0005](0005-macro32-on-the-calling-standard.md)) before code
generation starts here. BLISS's
default linkage is the calling standard; `JSB` linkages name VAX registers,
mapped as `vmacro` maps them.

**Listing.** `/LIST` writes the source with macro expansions
(`/SHOW=EXPANSIONS`), and with `/MACHINE_CODE`, the ARM64 each line became.
The listing is how a compiled routine is debugged.

## The dot lint

A data name in BLISS is its address and `.X` is its value, so a missing
or extra dot compiles silently. The compiler tags every expression as a
value, the address of a scalar, the address of an aggregate, or unknown,
and warns where they don't fit:

1. A conditional test of a scalar's address: `IF NOT STATUS THEN`.
2. Arithmetic or comparison on a scalar's address: `COUNT + 1`,
   `X GTR 5`.
3. A fetch from a `LITERAL` or a routine name: `.LIT`.
4. A system service or RTL argument passed the wrong way for its
   mechanism (by value, by reference, by descriptor), from a table
   generated with the definitions.
5. Storing a scalar's address: `Y = X`, where `X` is a plain scalar.
6. A formal used as a value in the callee and passed as an address by a
   caller, or the reverse.

Rules 1 to 4 come with the front end; 5 and 6 later. A line ending in
`! LINT: ADDRESS` silences it. CI compiles with warnings as errors.

## Self-hosting and bootstrap

- **Stage 0** is `vbliss`, in Rust. It is the system's BLISS-64 compiler
  until stage 1 passes its tests, and the bootstrap after that.
- **Stage 1** is `BLISS.EXE` built by stage 0. **Stage 2** is
  `BLISS.EXE` built by stage 1, and **stage 3** by stage 2. Stages 2 and 3
  must be identical byte for byte, so the compiler's output must not
  depend on time, host or hash order.
- Until stage 1 replaces it, every test runs through both compilers and
  their IR and assembly must match. Afterwards stage 0 is frozen: bug fixes
  only, and the compiler's own source may use only what stage 0 accepts.
  The bootstrap from source (`just bliss-bootstrap`: stage 0, then 1, 2
  and 3) runs in CI, so that rule is checked.
- **File I/O under `vrun`.** The compiler reads sources and require files
  and writes assembly and listings. `vrun`'s monitor calls cover only a
  console and an exit, and semihosting is out because it is an undefined
  instruction under HVF ([PRD-0001](0001-vtools.md)). `vrun` gains file
  monitor calls (open, read, write, close, on paths the driver allows),
  documented in `vtools/docs/runner-abi.md`, working under TCG and HVF.

## Testing strategy

**Oracle: listings.** For the front end, the reference compiler is exact.
A test compiles a source with BLISSA64 `/LIST/SHOW=EXPANSIONS` and with
`vbliss`, and compares macro expansions, compile-time values (`%PRINT`,
`%NUMBER`, `%FIELDEXPAND`), field offsets and which constructs are
diagnosed. Nothing there depends on the target.

**Oracle: behaviour.** The same test program compiled by BLISSA64 and run
on OpenVMS Alpha, and compiled by `vbliss` and run under `vrun`, prints the
same lines through a tiny print routine each side provides. Agents generate
these programs by the hundred for the semantics that are easy to get
wrong: field extension, signed and unsigned comparisons, `INCR`/`DECR`
bounds, `CASE` ranges, `SELECTONE` order, `LEAVE` values, `PLIT` layout.

**Running the oracle.** AXPbox runs on its own copy of the system disk
with BLISSA64 installed, never the user's playground disk, in batch through
`ods/vms/run-vms.py`. Its results are committed as expected outputs, so
CI never needs AXPbox. AXPbox hangs about once in six scripted runs; the
harness retries.

**Parse corpus.** The VAX/VMS V4.3 BLISS-32 sources, run through the
parser locally and never committed, find what the parser doesn't accept.
Compiled with `/A32`, they also show which VAX built-ins real code leans
on (`bliss64.md`, *BLISS-32 on vaxpunk*).

**The lint, measured.** False positives: lint warnings on the V4.3 corpus,
which DEC shipped and is mostly right. Recall: delete dots at random from
correct test programs and count what the lint catches. Both numbers go in
CI with a floor.

**Interop.** Programs that mix MACRO-32 and BLISS-64 modules under `vrun`:
calls each way with `CALL` and `JSB` linkages, a handler established in
BLISS-64 and a signal raised in MACRO-32, and the reverse.

**Bootstrap.** Stage 0 and stage 1 agree on every test; stages 2 and 3
are identical.

**The `vasm` port.** VASM assembles `vasm`'s whole test suite and the
assembly `vbliss` generates, and its OBJ must equal the Rust `vasm`'s byte
for byte. Its encoder is checked against GNU `as`, as `vasm`'s is.

**On vaxpunk.** A test in the boot suite runs `BLISS.EXE` on a small
module from the system disk, and its OBJ must equal the one the host made.

## Open questions

- **BLISS-64 documentation.** Answered in `bliss64.md`: there is no
  BLISS-64 manual; the kit's release notes (chapter 2, differences from
  BLISS-32) and the compiler's own tables are the sources, and what they
  leave open is listed there for the oracle to probe.
- **`vrun` file I/O.** Settled: the stub forwards the calls to `vrun` over
  the console UART, which QEMU offers under TCG and HVF alike, so no file
  has to be named up front (`vtools/docs/runner-abi.md`). It costs a QEMU
  exit per byte under HVF; a faster channel can come if compile times show
  it matters.
- **Precompiled libraries.** `LIBRARY` is how BLISS avoids reparsing
  STARLET. Whether `.L64` is a serialized symbol table or a cached
  expansion, and whether it's needed at all before compile times hurt.
- **Where BLISS-64 code goes first.** A new utility (AUTHORIZE), DCL's
  lexical functions, or LOGINOUT. The pilot decides whether BLISS-64 is
  worth it, so it should be a part of
  [PRD-0003](0003-multi-user-vms.md) with a MACRO-32 counterpart to compare
  against.
- **64-bit address space.** BLISS-64 code works below 2 GB like everything
  else until P2 space and `_64` services exist. Those belong to a separate
  ADR.
- **Speed.** How long the build may take with the compiler under `vrun`,
  under HVF on the Mac and under TCG in CI.
- **The `vasm` port's scope.** `BLISS.EXE` needs only the directives its
  own output uses. A full port, macros and `.LIBRARY` included, makes
  VASM a usable assembler on vaxpunk; the PRD asks for the full port, and
  this is where to cut if it drags.
- **Native linking.** Porting `vlink` and `vlib` to BLISS-64 makes the
  toolchain whole on vaxpunk. A PRD of its own, once the `vasm` port shows
  what such a port costs.

## Work order

Each step ends with something you can run or look at.

1. **Spec notes.** Read the Language Reference Manual and DEC's Alpha
   BLISS documentation; write `vtools/docs/bliss64.md` with the BLISS-64
   delta and its sources. *Visible:* the document.
2. **Calling standard.** [PRD-0005](0005-macro32-on-the-calling-standard.md):
   `vmacro`, the PAL and the executive moved to
   [ADR-0023](../adr/0023-calling-standard.md). *Visible:* every existing
   test and boot pass on the new standard.
3. **Oracle harness.** A script that compiles and runs a BLISS-64 program
   on its own AXPbox disk and returns the listing and the output.
   *Visible:* the oracle's output for `hello.b64`.
4. **First code.** Lexer, parser, routines, `OWN` and `LOCAL`, expressions,
   control, the IR and the back end. *Visible:* `vbliss hello.b64`,
   `vlink`, `vrun` prints hello, and `vbliss --ir` shows the IR.
5. **Macros and require files.** The macro language, lexical functions,
   `REQUIRE`, the listing with expansions. *Visible:* the listing tests
   pass against the oracle's listings.
6. **Structures and data.** `BLOCK`, `VECTOR`, `FIELD`, user structures,
   `BIND`, `PLIT`. *Visible:* the behaviour tests pass against the oracle.
7. **Definitions.** `vdefs`: `LIB.R64` and `STARLET.R64` from the macro
   libraries, stale check in CI. *Visible:* a BLISS-64 program reads a
   field of `$PCBDEF` at the same offset MACRO-32 does.
8. **Linkages and conditions.** `LINKAGE`, `BUILTIN`, `ENABLE`, `SIGNAL`.
   *Visible:* the interop tests under `vrun`.
9. **BLISS-32.** `/A32`, and `LONG_DEFAULT`, `REF_LONG` and
   `SIGNED_LONG` under `/A64`. *Visible:* the behaviour tests pass under
   `/A32` against BLISSA64's `BLISS/A32`, and a BLISS-32 module calls a
   BLISS-64 one and MACRO-32 under `vrun`.
10. **The lint.** Rules 1 to 4, with their numbers on the corpus and the
   mutation test. *Visible:* the report, and CI failing on a missing dot.
11. **Pilot.** One part of PRD-0003 written in BLISS-64, in the boot image.
    *Visible:* the system boots and its test passes. Stage 0 is now the
    system's BLISS-64 compiler.
12. **File I/O in `vrun`.** *Visible:* a test image under `vrun` reads a
    host file and writes another.
13. **Stage 1.** `vbliss` ported to BLISS-64 and built by stage 0.
    *Visible:* every test passes through stage 1 with output identical to
    stage 0's.
14. **The `vasm` port.** VASM in BLISS-64, built by stage 1. *Visible:*
    its OBJ equals the Rust `vasm`'s on the whole test suite, and
    `BLISS.EXE` writes OBJ modules without the host's `vasm`.
15. **Self-hosting.** Stages 2 and 3, the bootstrap job in CI, the build
    switched to `vbliss --stage1`, stage 0 frozen. *Visible:*
    `just bliss-bootstrap` ends with stages 2 and 3 identical.
16. **On vaxpunk.** The I/O module's RMS implementation, `BLISS.EXE` and
    `VASM.EXE` on the system disk. *Visible:* `$ BLISS HELLO` at the DCL
    prompt writes `HELLO.OBJ`, equal to the host's.
