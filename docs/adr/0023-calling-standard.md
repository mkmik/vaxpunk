# ADR-0023 — The vaxpunk calling standard is AAPCS64 with VMS's argument count, sign extension and self-describing frames, and MACRO-32 is compiled onto it as AMACRO compiled it onto Alpha's

Oct 6, 2026 · @Marko Mikulicic

Accepted. Every call, in any language, follows AAPCS64 with what 64-bit
VMS adds where AAPCS64 leaves room: argument information in x9, 32-bit
values sign-extended across calls, x15 for a bound procedure's
environment, x18 as the VAX stack pointer, frames that describe
themselves through a descriptor address at `24(FP)` next to a condition
handler at `16(FP)`, and VMS's 64-bit signal and mechanism arrays.
Interrupts and exceptions preserve every register in full. MACRO-32 is
compiled onto it the way DEC's MACRO-32 compiler (AMACRO) compiled it
onto Alpha's standard: routines declared, registers kept as sign-extended
longwords and preserved in full, `n(AP)` read from an argument list the
prologue homes at `32(FP)` when the routine uses AP, JSB a native call,
and DEC's 64-bit extensions under DEC's names.
The standard is [DESIGN-0004](../design/0004-calling-standard.md); what
DEC did is in `crosstools/vtools/docs/amacro.md`.

## Context

`vmacro` calls as the VAX did, a convention `crosstools/vtools/docs/macro32.md`
calls provisional. Three things need a real one: BLISS-64
([PRD-0004](../prd/0004-bliss64-compiler.md)) must call MACRO-32 and be
called by it; 64-bit code needs arguments that hold 64-bit addresses;
and C or Fortran, if they come, will come from compilers that speak
AAPCS64.

OpenVMS solved this three times (`crosstools/vtools/docs/amacro.md`). On Alpha, DEC
wrote the calling standard to fit VAX code: R2-R15 saved, so VAX R2-R11
stayed where they were. On Itanium and x86-64, VMS took the platform's
conventions and added what VMS needs: an argument count with the
floating point arguments' classes (R25, then %rax), sign extension of
every 32-bit value, an environment register, and the frame flags carried
into the unwind information. The MACRO-32 compiler followed each time:

- every routine declared, with `.CALL_ENTRY`, `.JSB_ENTRY` or
  `.JSB32_ENTRY`;
- registers kept as sign-extended longwords, and the full 64 bits of
  every register a routine modifies saved and restored, R0 and R1
  excepted, because a VAX routine's `PUSHR` keeps only longwords;
- `n(AP)` compiled to where the argument arrived, with the arguments
  copied into a VAX list ("homed") only when the code needs one;
- JSB a native call, its return address no longer on the stack, and
  code that played with it flagged;
- on Itanium, VAX registers remapped onto the hardware's to fit Intel's
  conventions, R0 and R1 onto the return registers;
- 64-bit use opt-in and explicit: `QUAD_ARGS`, quadword address
  arithmetic, the `$CALL64` macros and the `EVAX_` built-ins.

vaxpunk's provisional convention clashes with AAPCS64 at every point:
R2-R11 are in x2-x11, which a callee may destroy; arguments are in
memory; FP points at the handler with the caller's FP at `16(FP)`, so no
debugger walks the chain; VAX SP is x28, a saved register `vmacro` moves;
registers hold longwords zero-extended; and the PAL's frame keeps only
some registers, so a handler's `PUSHR` would cut the upper halves off
whatever 64-bit code it interrupted.

## Decision

1. **AAPCS64 is the base**: argument and result placement, saved and
   temporary registers, alignment, the frame record at `0(FP)`.
2. **x9 carries argument information**: the count in bits 7:0 and the
   classes of the first 28 arguments, two bits each, so a routine can read
   its arguments as a list when some came in floating point registers.
3. **32-bit values are sign-extended** as arguments and results, unsigned
   ones too.
4. **x15 carries a bound procedure's environment**, and **x18 is the VAX
   stack pointer**, AAPCS64's platform register, kept across calls.
5. **A procedure value is an entry address** that fits in 32 bits; code
   above 2 GB is reached through a linker trampoline. No procedure
   descriptors as values, no linkage sections.
6. **Frames describe themselves.** A prologue pushes the frame record,
   writes a clear handler at `16(FP)` and the frame descriptor's address at
   `24(FP)`, then sets FP. The descriptor, read-only and readable wherever
   the routine runs, holds Alpha's procedure descriptor flags, the saved
   registers and their place, the frame's size, a static handler and the
   routine's name.
   Frameless routines and JSB routines run on their caller's frame. Frames
   from other compilers get an unwind table keyed by PC, tried first.
7. **Condition handling is 64-bit VMS's**: 32-bit and 64-bit signal
   arrays, the 64-bit mechanism array with ARM64's registers, `$UNWIND`
   calling removed frames' handlers with `SS$_UNWIND`.
8. **Interrupts and exceptions preserve every register**, all 64 bits:
   the PAL's frame holds x0-x30 and `sp`, and `REI` restores all of them.
9. **`CHMx` carries its code with the PAL call**, in x7, and the PAL
   pushes it below the frame as the VAX did. The dispatcher checks that
   arguments of services not built for 64-bit addresses are sign-extended
   longwords, else `SS$_ARG_GTR_32_BITS`.
10. **MACRO-32 follows AMACRO.** R0-R1 in x0-x1, R2-R11 in x19-x28, SP in
    x18, AP the argument list at `32(FP)`. Every routine declared, with
    AMACRO's directives and parameters. A register always holds its
    longword sign-extended. Routines save the full 64 bits of what they
    modify. JSB and `RSB` are `bl` and `ret`. `n(AP)` reads a list the
    prologue fills only when the routine uses AP. DEC's 64-bit pieces keep
    DEC's names.

## Alternatives considered

| Option | Why not |
| --- | --- |
| Keep the VAX argument list in memory | Arguments stay 32-bit, so no P2 address fits; BLISS-64 would need a convention of its own to call MACRO-32; no other compiler could call either. |
| Transliterate Alpha's standard: six argument registers, procedure descriptors as values, linkage sections | Descriptors as values and linkage sections exist to reach data through Alpha's global pointer; ARM64 is PC-relative. Itanium and x86-64 VMS dropped them too. |
| Make x2-x11 saved registers, so R2-R11 stay put | What DEC did on Alpha, where DEC wrote the standard. Here it makes vaxpunk incompatible with every ARM64 compiler; Itanium VMS remapped instead, as this does. |
| Count in x8, or in memory, or an x86-64-style pointer to an argument information block | x8 is AAPCS64's indirect result register. A count in memory, or a block, costs a store or a constant per call; 56 free bits in x9 hold the classes of 28 arguments. |
| VAX SP as `sp` | AMACRO used Alpha's SP, and checked alignment only at calls. EL0 checks `sp`'s alignment on every access, and `PUSHL` leaves it 4-byte aligned. |
| Pseudo-registers in memory, as x86-64 VMS's MACRO-32 compiler | x86-64 has 16 registers; ARM64 has 31, enough to keep R0-R11 in real ones. |
| Keep JSB's return address on the VAX stack | `JSB @(SP)+` and return-address tricks would keep working, but no code here uses them, AMACRO flagged them, and a stack return address makes BLISS-64's JSB linkages maintain the VAX stack. |
| Honour the entry mask exactly, as the VAX did, without declarations | A register the routine changes but the mask leaves out, or a JSB routine's longword `PUSHR`, cuts a 64-bit caller's value in half, silently. |
| Save all of R2-R11 in every CALL routine that calls a JSB routine, instead of declaring JSB routines | Safe without touching the source, but costs most calls five more pair stores and loads, and leaves JSB routines callable only from code that knows they clobber. Kept for JSB calls across modules without a linkage declaration. |
| The register mask in the frame (ADR-0021) | The descriptor's address costs the same store and says more: flags, where each register is, the frame's size for `$UNWIND`, a static handler, the routine's name. |
| Unwind tables keyed by PC for every frame, as Itanium and x86-64 VMS | Linker and image work vaxpunk's own compilers don't need, since every frame they build names its descriptor. Kept for other compilers' frames. |
| The PAL saves only temporaries, handlers save the rest | Handlers save with longword `PUSHR`s, so 64-bit code they interrupt loses upper halves. Saving all also keeps kernel values out of an outer mode's registers after `CHMx`. |

## Consequences

**What gets harder.**
- `vmacro` changes in its register map, its calls, entries and returns,
  its handling of longwords (sign-extended) and its temporaries, and
  learns AMACRO's directives and DEC's 64-bit pieces
  ([PRD-0005](../prd/0005-macro32-on-the-calling-standard.md)).
- The tree's 360 JSB routines get `.JSB_ENTRY` or `.JSB32_ENTRY`
  declarations with their `output` and `scratch` registers, from the
  register contracts their comments already state. A wrong declaration
  either restores a register a caller wanted, or loses one it kept.
- Keeping registers sign-extended costs an instruction after some
  longword operations.
- The PAL's frame grows from 12 to 34 quadwords, saved and restored on
  every interrupt and exception.
- ADR-0021's frame and mechanism array change: the handler moves from
  `0(FP)` to `16(FP)`, the caller's FP to `0(FP)`, the mask at `40(FP)`
  gives way to the descriptor at `24(FP)`, and the mechanism array becomes
  64-bit. ADR-0021 is updated in place, as it is still Proposed.
- `SYS$name`, `EXE$CMODKRNL`, `EXE$CMODEXEC`, `ARGLIST`, `EXE$SIGNAL`,
  `SYS$UNWIND`, `LIB$SIGNAL` and `LIB$ESTABLISH` change by hand.

**What stays easy.**
- MACRO-32 reads as on the VAX: `4(AP)`, entry masks, `CALLS`, `CALLG`,
  `JSB`, `MOVAB handler, (FP)`. Sources DEC's customers ported to Alpha,
  declarations and 64-bit pieces included, compile as they are.
- BLISS-64 calls MACRO-32 with its default linkage and is called by it,
  and calls JSB routines through `JSB` linkages that match their
  declarations.
- `vrun --gdb` backtraces through every frame.
- A C or Fortran compiler needs a few things it can be told or taught
  (DESIGN-0004, *C and Fortran*), not a different ABI.

**Follow-ups:** the 64-bit address space (P2 and S2, `_64` services,
64-bit pool), in an ADR of its own; the unwind table format, when another
compiler's code arrives; `LIB$GET_CURR_INVO_CONTEXT` and the invocation
context routines; AST and exception frame flags in the frames the
executive builds.
