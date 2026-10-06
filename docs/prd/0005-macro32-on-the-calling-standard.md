# PRD-0005 — MACRO-32 on the vaxpunk calling standard, as AMACRO put it on Alpha's

Oct 6, 2026 · @Marko Mikulicic

## Context and goal

vaxpunk's executive, drivers and utilities are about 25,000 lines of
MACRO-32 in 96 files: 151 `.ENTRY` routines, 360 JSB routines reached from
1225 `JSB` and `BSBx` sites, 192 `PUSHR` and `POPR`. `vmacro` compiles them
with a provisional convention: VAX argument lists in memory, R2-R11 in
x2-x11, longwords zero-extended, a frame only MACRO-32 understands.
[ADR-0023](../adr/0023-calling-standard.md) settles the real one and
[DESIGN-0004](../design/0004-calling-standard.md) specifies it.

DEC did this in 1992, and again for Itanium and x86-64
(`vtools/docs/amacro.md`). AMACRO compiled the VAX executive onto the
Alpha calling standard: routines were declared, registers kept 64 bits
for callers in other languages, `4(AP)` still read the first argument,
and V7.0 added the pieces that let MACRO-32 handle 64-bit addresses where
it must. The executive stayed MACRO-32, and no other compiler was needed
to build it.

The goal is the same, and it comes before BLISS-64
([PRD-0004](0004-bliss64-compiler.md)): **the calling standard settled
and in force, and the low-level tools to work with it.** Concretely:

- `vmacro` compiles the tree onto DESIGN-0004, speaking AMACRO's dialect:
  its routine declarations and their parameters, its register
  preservation, its argument rules, the idioms it flags.
- DEC's 64-bit pieces work under DEC's names: `QUAD_ARGS`, quadword
  address arithmetic, the `$CALL64` macros, `EVAX_CALLG_64`, the test
  macros and the `EVAX_` built-ins that mean something on ARM64.
- The PAL, the executive's change mode, signal and unwind code, and `vrun`
  follow the standard, with 64-bit VMS's signal and mechanism arrays.
- Hand-written ARM64 can build standard frames with a macro library, and
  `vrun` prints a traceback through any frames that follow the standard.

Then BLISS-64, or any conforming compiler, calls any declared MACRO-32
routine and is called by one, with nothing in between.

## Non-goals

- **No 64-bit address space.** P2 and S2, `_64` services and 64-bit pool
  are an ADR of their own. The 64-bit pieces here work on 64-bit values
  and pointers wherever they come from.
- **No BLISS-64, C or Fortran.** Hand-written ARM64 stands in for them in
  tests.
- **No source rewrite.** `.mar` files gain declarations; other changes
  are the few listed under *What changes by hand*.
- **No unwind tables.** Our frames name their descriptors; tables keyed by
  PC wait for another compiler.
- **No new optimizations** beyond what AMACRO's rules give for free (no
  argument list unless AP is used, no saves of what isn't modified).
  Recognising the pushes before `CALLS` and loading registers directly, as
  AMACRO did, comes later if calls cost too much.

## What `vmacro` does

Everything in DESIGN-0004's *MACRO-32* and *JSB* sections, from
`vtools/docs/amacro.md`'s *What vmacro takes*. In short:

**Registers.** R0 and R1 in x0 and x1, R2-R11 in x19-x28, SP in x18, FP
in x29; AP is the argument list at `32(FP)`, x12 in a routine that writes
AP. Temporaries x2-x8, x10-x17 and x30. A VAX register always holds its
longword sign-extended, as AMACRO kept it.

**Declarations.** `.CALL_ENTRY`, `.JSB_ENTRY` and `.JSB32_ENTRY`, with
`max_args`, `home_args`, `quad_args`, `input`, `output`, `scratch`,
`preserve` and `label`; `.ENTRY` as a `.CALL_ENTRY` with a mask;
`.GLOBAL_LABEL`. `.CALL_LINKAGE`, `.DEFINE_LINKAGE` and `.USE_LINKAGE`
say what a JSB routine in another module modifies. Every `CALLS`,
`CALLG`, `JSB` and `BSBx` target, every branch target in another routine
and every stored code address is declared; code outside a routine is an
error.

**Preservation.** The full 64 bits of each register a routine modifies,
R0 and R1 excepted, unless `output` or `scratch`; `preserve` always.
`.JSB32_ENTRY` preserves nothing undeclared. A CALL routine counts as
modified what its JSB routines in the module declare `output` or
`scratch`, and all of R2-R11 for a JSB into another module without a
linkage directive.

**Arguments.** The argument list at `32(FP)`, filled by the prologue only
when the routine uses AP: up to the highest fixed `n(AP)`, or up to the
count when AP is used as an address, indexed or offset by a variable, or
with `home_args=TRUE`. Quadwords with `quad_args`. `CALLS` and `CALLG`
load x0-x7 and the stack, sign-extended, and set x9.

**JSB** is `bl`, `RSB` is `ret`; a JSB routine that calls saves x30 with
the registers it preserves.

**Frames.** DESIGN-0004's prologue, frame and descriptor in `$LINK$`;
`(FP)` and `0(FP)` as the handler slot.

**64-bit.** `quad_args`, `.ENABLE QUADWORD`, `.DISABLE QUADWORD` and
`/ENABLE=QUADWORD`; `EVAX_CALLG_64`; and these built-ins, each compiled to
the ARM64 that does its job:

| Group | Built-ins |
| --- | --- |
| Sign extension | `EVAX_SEXTB`, `SEXTW`, `SEXTL` |
| Loads and stores | `EVAX_LDQ`, `STQ`, `LDAQ`, `LDBU`, `LDWU`, `STB`, `STW`, `LDQU`, `STQU` |
| Locked | `EVAX_LDQL`, `STQC`, `LDLL`, `STLC` (exclusive load and store) |
| Arithmetic | `EVAX_ADDQ`, `SUBQ`, `MULQ`, `UMULH` |
| Logic and shifts | `EVAX_AND`, `OR`, `XOR`, `BIC`, `ORNOT`, `EQV`, `SLL`, `SRL`, `SRA`, `ZAP`, `ZAPNOT` |
| Compare, branch, move | `EVAX_CMPEQ`, `CMPLT`, `CMPLE`, `CMPULT`, `CMPULE`, `BEQ`, `BLT`, `BNE`, `CMOVxx` |
| Barrier | `EVAX_MB` |
| PAL | `EVAX_MTPR_x` and `EVAX_MFPR_x` for the processor registers the PAL has (`$PRDEF`) |

The rest of Alpha's list (byte manipulation, `TRAPB`, `RPCC`, the FPCR,
the PAL calls with no vaxpunk meaning) is an error naming the built-in.

**Porting messages**, as AMACRO's, each with the line: an undeclared
target; a register written that the mask leaves out; a positive FP offset
other than the handler (an error); a JSB routine that reads its caller's
argument list; AP written and then read through; popping, pushing or
rewriting a return address, and `JSB @(SP)+` to an undeclared target (an
error); a branch into another routine; raw ARM64 naming x2-x30.

## The libraries

| Library | Gains |
| --- | --- |
| `lib.mlb` | `$FDSCDEF`, `$INTSTKDEF` for the full frame |
| `starlet.mlb` | `$CHFDEF` with the 64-bit mechanism and signal arrays; `$DSCDEF` with `DSC64$`; the 64-bit item list fields; `SS$_SIGNAL64`, `SS$_ARG_GTR_32_BITS`, `SS$_UNWIND`; `$SETUP_CALL64`, `$PUSH_ARG64`, `$CALL64`, `$IS_32BITS`, `$IS_DESC64`, `$PUSH64`, `$POP64` |
| `call.mlb`, new | `$ROUTINE`, `$RETURN` and `$CALL` for hand-written ARM64 in `vasm`: a standard prologue, epilogue and descriptor, and a call that sets x9, as MACRO-64's macros did on Alpha |

## What changes by hand

| Where | What |
| --- | --- |
| The tree's `.mar` files | `.JSB_ENTRY` or `.JSB32_ENTRY` on each of the 360 JSB routines, `output` and `scratch` from the register contract its comment states (AGENTS.md's "Uses Rn."); `.GLOBAL_LABEL` where needed; `home_args=TRUE` on callers whose JSB routines read AP |
| `roottask/src/main.c` | The PAL's frame holds the PC, the PSL, x0-x30 and `sp`; `REI` restores them all; `CHMx` takes its code from x7's bits 31:16 and pushes it below the frame; delivery to an inner mode sets its x18 and `sp` to the frame |
| `roottask/exec/syssrv.mar` | `SYS$name` frameless, moving x7 to x10 for services with eight or more arguments; `EXE$CMODKRNL` and `EXE$CMODEXEC` pop the code, check x9, copy arguments past the eighth from the caller's stack, check sign extension, call `EXE$name`, and write x0 and x1 into the frame; `ARGLIST` goes |
| `roottask/exec/sysunwind.mar` | `EXE$SIGNAL` and `SYS$UNWIND` walk `0(FP)`, find handlers at `16(FP)` and descriptors at `24(FP)`, build 64-bit VMS's arrays, call removed frames' handlers with `SS$_UNWIND`, and restore registers and `sp` from descriptors |
| `roottask/sysexe/lib/signal.mar` | `LIB$SIGNAL` and `LIB$ESTABLISH`: the same frame |
| Handlers in the tree | Read the mechanism array's 64-bit fields |
| `roottask/sysexe/spin.mar` | Fills all 64 bits of every register it checks, with built-ins |
| `vtools/crates/vrun` stub | Enters an image with x9 = 1 and x18 = `sp`; on a fault, walks FP and prints each frame's PC and routine name from its descriptor |
| `vtools/docs/macro32.md`, `runner-abi.md`, DESIGN-0001, ADR-0021 | Follow the code |

## Testing strategy

**Everything that runs now keeps running.** `cargo test` for vtools, the
boot test and every test image (`svctest`, `chftest`, `asttest`,
`mbxtest`, `spin`...) pass after each step, with no change to their
expected output except register dumps and tracebacks.

**64-bit callers.** `vtools/tests/run` programs whose callers are
hand-written ARM64 built with `call.mlb`, standing in for BLISS-64. One
fills x18-x28 with 64-bit patterns and calls MACRO-32 routines that use
`PUSHR`, call `.JSB_ENTRY` and `.JSB32_ENTRY` routines, write registers
outside their mask, and call back into ARM64; every pattern comes back
whole except declared outputs.

**Declarations.** A test per parameter (`output`, `scratch`, `preserve`,
`.JSB32_ENTRY`'s barrier), and `vdump` showing that a routine saves
exactly what it modifies.

**Arguments.** Calls with 0, 1, 8, 9 and 255 arguments through `CALLS`
and `CALLG`, read through `n(AP)`, through AP as an address and through
registers; negative longwords arriving sign-extended; a JSB routine
reading its caller's list; `quad_args` and `EVAX_CALLG_64` with 64-bit
values.

**64-bit.** Each built-in against what DEC's guide says it does; the
guide's `MOVAL (R1)[R0], R2` wrap example in longword and quadword mode;
the guide's `$SETUP_CALL64` example and one with 9 arguments; `$IS_32BITS`
and `$IS_DESC64` on both forms.

**Conditions.** A handler established with `MOVAB handler, (FP)` catches
a fault raised in an ARM64 routine it called; `$UNWIND` through an ARM64
frame built with `$ROUTINE` restores its registers; removed frames'
handlers see `SS$_UNWIND`; handlers read the 64-bit mechanism array.

**Interrupts.** `spin` under the boot test, with every register holding a
64-bit pattern across ticks, `CTRL/Y` and `CONTINUE`.

**Messages.** `crates/vmacro/tests/errors.rs` gains a case per porting
message.

**Tracebacks.** A `vrun` test that faults three calls deep prints the
three routines' names; `vrun --gdb` and LLDB's `bt` list the same frames.

## Open questions

- **Cost.** The PAL's frame grows from 12 to 34 quadwords; registers are
  sign-extended after some longword operations; JSB calls into other
  modules make their callers save all of R2-R11. Whether boot time or
  `svctest` notices.
- **Declarations by hand or by tool.** Settled in step 3: a script drafted
  them from the contracts' "Uses Rn." and "Keeps every register.", a
  routine without one became `.JSB32_ENTRY`, then `.JSB_ENTRY` with
  `scratch` what it modifies and doesn't pop back; each routine that then
  restored a register it writes was reviewed for an output the contract
  left out.
- **Linkage directives across modules.** Settled in step 3: neither. The
  build compiles the executive's modules together, and the programs' and
  their libraries' together (`vmacro::compile_modules`), and each module
  gets the others' declarations as linkages.

## Work order

Each step ends with every existing test and the boot passing.

1. **The PAL keeps every register.** Its frame holds x0-x30 and `sp`,
   `REI` restores them all, and the change mode handlers write their
   results into it. *Visible:* `spin`, its upper halves set with raw ARM64,
   survives ticks, `CTRL/Y` and `CONTINUE`.
2. **Registers.** R2-R11 to x19-x28, SP to x18, the temporaries moved,
   longwords kept sign-extended. Calls still pass VAX argument lists.
   *Visible:* a register dump shows R2 in x19, and `MNEGL #1, R0` leaves
   x0 all ones.
3. **Declarations.** AMACRO's directives and preservation in `vmacro`, the
   tree declared, undeclared targets an error. *Visible:* the declaration
   and 64-bit caller tests pass, and the tree builds with no porting
   messages.
4. **Frames and conditions.** DESIGN-0004's frame and descriptors, the
   handler idiom, the 64-bit signal and mechanism arrays, `sysunwind.mar`
   and `signal.mar` by hand, `call.mlb`, and `vrun`'s traceback.
   *Visible:* `chftest` and the condition tests pass, and a faulting test
   prints its traceback.
5. **Arguments and calls.** `CALLS`, `CALLG`, x9, the argument list at
   `32(FP)`, JSB as `bl` and `ret`, `home_args=TRUE` on `SYS$name`, and
   `vrun`'s entry. *Visible:* the argument tests pass.
6. **System services.** `CHMx` with its code in x7, frameless `SYS$name`,
   the dispatchers on registers with the sign-extension check. *Visible:*
   `svctest` passes, and a test passing a 64-bit address to `$QIOW` gets
   `SS$_ARG_GTR_32_BITS`.
7. **64-bit pieces.** `quad_args`, quadword arithmetic, the built-ins, the
   `$CALL64` macros, `EVAX_CALLG_64` and the test macros. *Visible:* the
   64-bit tests pass, the guide's examples among them.
8. **Documents.** `macro32.md` (with the built-in table), `runner-abi.md`,
   DESIGN-0001, ADR-0021 and DESIGN-0004 match the code; ADR-0023
   accepted. *Visible:* the documents.
