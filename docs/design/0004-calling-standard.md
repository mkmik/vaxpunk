# DESIGN-0004 — The vaxpunk calling standard

Oct 6, 2026 · @Marko Mikulicic

This is the calling standard every routine on vaxpunk follows, in any
language: how arguments and results travel, which registers survive a
call, what a frame looks like, and how condition handling finds its way
through frames. [ADR-0023](../adr/0023-calling-standard.md) records why it
is this way, and `vtools/docs/amacro.md` what OpenVMS did on Alpha,
Itanium and x86-64, which this follows. The last sections say how
MACRO-32 ([PRD-0005](../prd/0005-macro32-on-the-calling-standard.md)),
BLISS-64 ([PRD-0004](../prd/0004-bliss64-compiler.md)), C and Fortran map
onto it.

The base is Arm's Procedure Call Standard for the Arm 64-bit Architecture
(AAPCS64). Everything AAPCS64 says holds unless a section here says
otherwise. What this document adds is what VMS needs and AAPCS64 leaves
to the platform, as OpenVMS added it to Intel's conventions on Itanium
and x86-64: argument information, sign-extended 32-bit values, an
environment register, a frame that says how to unwind it, and VMS's
condition handling.

## Registers

| Register | Use | Across a call |
| --- | --- | --- |
| x0-x7 | arguments 1-8 of the general class; x0 the result, x1 its high half | not kept |
| x8 | AAPCS64's indirect result location | not kept |
| x9 | argument information, at entry only (*Arguments*) | not kept |
| x10-x14 | temporaries | not kept |
| x15 | the environment of a bound procedure, at entry only; else a temporary | not kept |
| x16, x17 | temporaries a linker veneer may change between a call and its target | not kept |
| x18 | the VAX stack pointer (*Stacks*) | kept |
| x19-x28 | saved registers; MACRO-32's R2-R11 | kept |
| x29 | FP, the frame pointer | kept |
| x30 | LR, the return address | not kept |
| sp | the stack pointer | kept |
| v0-v7 | arguments 1-8 of the floating point classes; v0 the result | not kept |
| v8-v15 | the low 64 bits saved, as in AAPCS64 | kept |
| v16-v31 | temporaries | not kept |

"Kept" means kept in full, all 64 bits, by the callee.

## Values

A 32-bit value in a 64-bit register or stack slot is sign-extended
wherever it crosses a call as an argument or a result: bit 31 is copied
into bits 32-63, for unsigned longwords too, as on every 64-bit VMS. A
32-bit address is therefore a 64-bit one, and a 64-bit address is a valid
32-bit one only when its upper 33 bits are all equal. Inside a routine, a
compiler keeps values as it likes.

## Arguments

**Placement** is AAPCS64's: arguments of the general class (integers,
addresses, anything passed by reference or by descriptor) in x0-x7, those
of the floating point classes in v0-v7, the rest on the stack in argument
order, 8 bytes each from `sp` up (16 for a 128-bit value). Aggregates by
value follow AAPCS64, but a routine whose arguments must be readable as a
list doesn't take them.

**Argument information.** The caller sets x9 on every call:

| Bits | Holds |
| --- | --- |
| 7:0 | the number of arguments, 0-255 |
| 63:8 | the class of arguments 1-28, two bits each from bit 8: 0 general, 1 single (S), 2 double (T), 3 quad (X, 128-bit) |

A class past the last argument is 0. An argument past the 28th must be of
the general class if the callee reads its arguments as a list. VAX
floating point values passed by value are general. With no floating point
arguments x9 is just the count, as Alpha's and Itanium's R25 is.

**Argument lists.** A routine that reads its arguments by position rather
than by name (MACRO-32's AP, BLISS-64's `ACTUALPARAMETER`, a routine with
optional arguments) saves x9 at entry and copies its arguments into a list
of its own: AAPCS64 doesn't put the register arguments next to the stack
ones, any more than x86-64 does. `CALLG` goes the other way, and
`LIB$CALLG` will.

**Mechanisms** are VMS's: by value, by reference, by descriptor. An
omitted argument is passed as 0.

## Results

x0 holds a general result, sign-extended if it is 32 bits; x0 and x1 a
128-bit one. v0 holds a floating point result. A larger result goes
through a hidden first argument, counted in x9. Other registers carry
results only in a linkage that says so (*JSB*, MACRO-32's `output`).

## Stacks

`sp` is 16-byte aligned at every call and whenever it addresses memory,
as AAPCS64 and EL0's alignment check require.

x18 is the VAX stack pointer, AAPCS64's platform register: MACRO-32 pushes
and pops longwords on it. It points into the same stack as `sp`, and `sp`
is never above it when MACRO-32 calls out. Code that doesn't use the VAX
stack leaves x18 alone; code that changes it restores it before
returning. The system sets it to `sp` where a mode's code starts. The live
part of the stack is everything at or above the lower of `sp` and x18;
only the PAL, when it delivers an interrupt or an exception, and the
executive, when it reflects an exception into the mode that took it
(ADR-0021), write below that, and both go below both.

## Procedure values

A procedure value is the routine's entry address. It must fit in 32 bits,
sign-extended, since MACRO-32 stores code addresses in longwords: code
above 2 GB will be reached through a trampoline the linker places below,
as on x86-64 VMS; nothing is linked there yet. There are no procedure descriptors as values and no linkage
sections; ARM64 reaches code and data PC-relative. A bound procedure
value points at a trampoline that loads x15 with the environment and
branches to the routine.

## Frames

A routine called by this standard has a frame if it calls another
routine, keeps a register from x18-x28 or v8-v15, or establishes a
condition handler. A routine without one is a frameless routine: it uses
only registers that aren't kept, leaves x18-x30 and `sp` alone, and is
never current, as an Alpha null frame procedure. A fault in it belongs to
its caller's frame.

The prologue builds the frame in this order:

1. `stp x29, x30, [sp, #-N]!`, or `sub sp, sp, #N` and `stp x29, x30,
   [sp]` for an N past 504: the frame record, AAPCS64's, at the bottom of
   the N bytes the frame takes, N a multiple of 16.
2. `adrp` and `add` x16 to the frame descriptor, then
   `stp xzr, x16, [sp, #16]`: a clear handler and its address.
3. `mov x29, sp`: from here FP is the new frame.
4. The kept registers it changes, where its descriptor says.

| Offset from FP | Holds |
| --- | --- |
| 0 | the caller's FP |
| 8 | the return address |
| 16 | the condition handler, 0 if none |
| 24 | the frame descriptor's address |
| 32 | MACRO-32's argument list, when the routine has one |
| after it | the save area, at the descriptor's offset, and the rest of the frame |

FP moves only at step 3, after `16(FP)` and `24(FP)` hold their values, so
a fault in a prologue, a stack overflow at step 1, finds FP still at the
caller's complete frame. Until FP moves, the saved registers still hold
the caller's values, as Alpha's standard requires. The epilogue loads the
saved registers, then `mov sp, x29`, `ldp x29, x30, [sp], #N` (or `ldp`
and `add sp, sp, #N`) and `ret`.

FP is a chain: each frame's `0(FP)` is its caller's FP. An FP of 0 ends
it; the system clears FP where a mode's code starts (`EXE$USRSTART`,
`EXE$CLISTART`, `EXE$ASTDISP`, ADR-0021).

### The frame descriptor

Each routine with a frame has a frame descriptor, `$FDSCDEF`, a 32-byte
block, read-only, readable in every mode that runs the routine: vmacro
puts it in an `EXE` psect named for the code's with `_FDSC` after, and
`call.mlb`'s `$ROUTINE` in `$CODE$_FDSC`, since an executive routine in
the vector runs in modes that can't read the executive's `$LINK$`:

| Offset | Field | Holds |
| --- | --- | --- |
| 0 | `FDSC$L_FLAGS` | `FDSC$V_HANDLER` (bit 0): `FDSC$Q_HANDLER` is a static handler; `FDSC$V_BASE_FRAME` (1): the chain ends here; `FDSC$V_TARGET_INVO` (2): call the handler when this frame is the target of an unwind; `FDSC$V_EXCEPTION_FRAME` (3): a frame exception delivery built; `FDSC$V_AST_FRAME` (4): a frame AST delivery built. Bits 1-4 are reserved: nothing sets or reads them yet |
| 4 | `FDSC$L_SAVED` | the kept registers saved: bit n for x18+n, n 0-10; bit 11+n for d8+n, n 0-7 |
| 8 | `FDSC$L_RSA` | the save area's offset from FP: the saved registers in ascending order, x before d, 8 bytes each |
| 12 | `FDSC$L_SIZE` | N: the caller's `sp` at the call is FP + N |
| 16 | `FDSC$Q_HANDLER` | the static handler, if `FDSC$V_HANDLER` |
| 24 | `FDSC$Q_NAME` | the routine's name, `.ASCIC`, as its offset from the descriptor, so that a position-independent psect can hold it, or 0, for tracebacks |

The flags are Alpha's procedure descriptor flags, kept as Itanium and
x86-64 VMS kept them in their unwind information.

### Whose frame

To search or unwind, the walker needs each frame's descriptor: for the
frame at FP, `24(FP)`, as an Alpha stack frame was known by the
descriptor at its `0(FP)`. Frameless routines and JSB routines run on
their caller's frame and need none.

**Frames from other compilers.** A compiler that can't build this frame
(a stock C compiler) describes its routines in an unwind table keyed by
PC, which the linker gathers per image, as Itanium and x86-64 VMS do for
every routine. The walker looks the PC in each frame up there first and
uses `24(FP)` if there is no entry. The table's format is defined when
such a compiler arrives; vaxpunk's own compilers never need one.

## Condition handling

**Handlers.** A frame's handler is `FDSC$Q_HANDLER` if `FDSC$V_HANDLER`
is set, else `16(FP)` if it isn't 0. The prologue clears `16(FP)`; a
routine establishes a handler by writing it there (MACRO-32's
`MOVAB handler, (FP)`, BLISS-64's `ENABLE`), or `LIB$ESTABLISH` does for
its caller. A handler is called by this standard with two arguments, the
signal and mechanism arrays, by reference. 64-bit VMS's primary,
secondary and last-chance vectors aren't here yet: the frames are
searched, and a condition no handler takes goes to `EXE$CATCHALL`.

**Signal arrays** come in two forms, as on 64-bit VMS: the 32-bit one, a
longword count, the condition, its arguments, the PC and the PSL, the PC
its low 32 bits; and the 64-bit one, a longword count, `SS$_SIGNAL64`,
then quadwords: the condition, its arguments, the PC and the PSL. A
handler tells them apart by the second longword.

**The mechanism array**, `$CHFDEF`, is 64-bit VMS's, with ARM64's
registers:

| Offset | Field | Holds |
| --- | --- | --- |
| 0 | `CHF$IS_MCH_ARGS` | quadwords after the first, 24 |
| 4 | `CHF$IS_MCH_FLAGS` | bit 0: floating point registers saved |
| 8 | `CHF$PH_MCH_FRAME` | the establisher's FP |
| 16 | `CHF$IS_MCH_DEPTH` | the establisher's depth: 0 the frame that signaled, 1 its caller, and so on out; −1, −2, −3 are for the vectors |
| 20 | | reserved |
| 24 | `CHF$PH_MCH_DADDR` | handler data, 0 |
| 32 | `CHF$PH_MCH_ESF_ADDR` | the exception's frame, 0 for a software signal |
| 40 | `CHF$PH_MCH_SIG_ADDR` | the 32-bit signal array |
| 48 | `CHF$IH_MCH_RETVAL` | x0 |
| 56 | `CHF$IH_MCH_RETVAL2` | x1 |
| 64-184 | `CHF$IH_MCH_SAVX2`... `SAVX17` | x2-x17 at an exception; 0 for a software signal |
| 192 | `CHF$PH_MCH_SIG64_ADDR` | the 64-bit signal array |

The kept registers are in the frames, as Alpha's mechanism array leaves
R2-R15 "implicitly saved in the call stack". A handler may change only
the return values, by writing `CHF$IH_MCH_RETVAL` and `RETVAL2`;
`SYS$SET_RETURN_VALUE` isn't here yet.

**Unwinding.** `$UNWIND(depadr, newpc)`, depadr the address of a longword
depth or 0 for the establisher's caller, removes frames when the handler
returns, calling each removed frame's handler with `SS$_UNWIND` first;
the target's, for `FDSC$V_TARGET_INVO`, isn't called yet. To remove a
frame, the unwinder loads every x register its descriptor says it saved
from its save area (nothing saves d8-d15 yet), sets `sp` to FP +
`FDSC$L_SIZE`, and FP and the return address from the record.

## Descriptors and item lists

Descriptors are VMS's, in two forms. A routine that takes both tells them
apart by testing both fields of the 64-bit form, as `$IS_DESC64` does,
since a 32-bit descriptor of length 1 passes the MBO test alone, and one
whose pointer is −1 the MBMO test; the executive's services take only the
32-bit form so far:

| Offset | 32-bit form | 64-bit form |
| --- | --- | --- |
| 0 | word `DSC$W_LENGTH` | word `DSC64$W_MBO` = 1 |
| 2 | byte `DSC$B_DTYPE`, byte `DSC$B_CLASS` | byte `DSC64$B_DTYPE`, byte `DSC64$B_CLASS` |
| 4 | longword `DSC$A_POINTER` | longword `DSC64$L_MBMO` = −1 |
| 8 | | quadword `DSC64$Q_LENGTH` |
| 16 | | quadword `DSC64$PQ_POINTER` |

Item lists add `item_list_64a` and `item_list_64b` (`$ILEDEF`; no
service takes them yet), recognised the same
way: word MBO = 1, word item code, longword MBMO = −1, quadword buffer
length, quadword buffer address, and in the `b` form a quadword return
length address. Other structures that carry addresses identify their
form the same way, or with an MBMO longword in place of the 32-bit field.

## Interrupts and exceptions

An interrupt or exception preserves every register of the code it stops,
all 64 bits. The PAL's frame (`$INTSTKDEF`) holds the PC, the PSL, x0-x30
and `sp`, and `REI` restores all of them from it. A handler may still
save what it uses, as VAX handlers do; it no longer has to. A handler
that hands back a register, a `CHMx` handler's status, writes it into the
frame before `REI`.

## System services and PAL calls

A PAL call is an `svc` with the function code in x7, as seL4 requires.
`CHMx` carries its change mode code with it, in x7's bits 31:16, and the
PAL pushes the code below the frame it delivers, as the VAX's `CHMx`
pushed it, so x0-x6 and x9 reach the handler untouched.

A program calls `SYS$name` like any routine. A `SYS$name` that changes
mode is a frameless routine in the vector: it moves x7, its eighth
argument if it has one, to x10, then changes mode. `SYS$EXIT` has a frame,
since it calls the exit handlers, and `SYS$UNWIND` and `SYS$PUTMSG` run
in the caller's mode. The dispatcher (`EXE$CMODKRNL`, `EXE$CMODEXEC`)
keeps the arguments, x10's for x7, and x9's count, at most 12, in
registers `REI` restores from the frame; pops the code; checks it, and
the count against the service's least; checks that each argument of a
service not built for 64-bit addresses is a sign-extended longword, else
returns `SS$_ARG_GTR_32_BITS`, as 64-bit VMS does; probes and copies the
arguments past the eighth from the caller's stack (`sp` in the frame) to
its own, checking them alike; puts them back in x0-x7 and x9; calls
`EXE$name` by this standard; writes x0 and x1 into the frame; and `REI`s.

## JSB

A JSB routine is MACRO-32's: declared with `.JSB_ENTRY` or
`.JSB32_ENTRY`, it runs on its caller's frame, takes and returns values in
VAX registers by its declaration, and returns with `RSB`. Only MACRO-32
and BLISS-64's `JSB` linkages call one.

- `JSB` and `BSBx` are `bl`, and `RSB` is `ret`, as AMACRO made them
  native calls: the return address is in x30, not on the VAX stack. A JSB
  routine that calls saves x30 itself, on `sp`, with the registers it
  preserves.
- A `.JSB_ENTRY` routine keeps all 64 bits of every register it modifies
  except R0, R1 and those it declares `output` or `scratch`, so a caller
  in any language gets its x19-x28 back but for those. A `.JSB32_ENTRY`
  routine keeps none unless declared `preserve`; only MACRO-32 calls it.

## MACRO-32

`vmacro` compiles MACRO-32 onto the standard as AMACRO compiled it onto
Alpha's (`vtools/docs/amacro.md`).

| VAX | ARM64 |
| --- | --- |
| R0, R1 | x0, x1 |
| R2-R11 | x19-x28 |
| AP | the argument list at `32(FP)`, in x12 where the routine reads it |
| FP | x29 |
| SP | x18 |

`vmacro`'s temporaries are x2-x11, x13-x17 and x30, which MACRO-32 can't
name; x12 holds AP where the routine reads it. A VAX register always holds its longword sign-extended, as on
Alpha: a longword instruction that writes a register leaves bits 32-63
equal to bit 31, and a byte or word write changes the low byte or word and
sign-extends from bit 31.

- **Declarations.** Every routine is declared: `.ENTRY` or `.CALL_ENTRY`
  for CALL routines, `.JSB_ENTRY` or `.JSB32_ENTRY` for JSB routines, with
  AMACRO's parameters (`max_args`, `home_args`, `quad_args`, `input`,
  `output`, `scratch`, `preserve`; `label` is accepted and ignored), and
  `.EXCEPTION_ENTRY` for code the PAL, `REI` or a jump enters, which has no
  frame and saves nothing. The target of every `CALLS`, `CALLG`, `JSB` and
  `BSBx` in the module is a declared entry, and that of a branch from
  another routine a declared entry or a `.GLOBAL_LABEL`. Code outside a
  routine is an error.
- **Preservation.** A CALL routine saves x18 and all 64 bits of the
  registers among R2-R11 that are in its mask or that it modifies, except
  `output` and `scratch`; a register its JSB routines in the same module
  declare `output` or `scratch` counts as modified; a `JSB` to a routine in
  another module counts as modifying all of R2-R11, unless a
  `.CALL_LINKAGE` or `.USE_LINKAGE` says what it modifies, or the modules
  are compiled together, which gives each the others' declarations as
  linkages. A CALL routine with nothing to save, no argument list to
  home, no calls and no use of SP or FP is frameless (*Frames*). JSB
  routines preserve as *JSB* says.
- **Arguments.** `n(AP)` reads the argument list at `32(FP)`: a count
  longword and one longword per argument, or quadwords with `quad_args`.
  The prologue fills it, from x0-x7, the caller's stack and x9, when the
  routine refers to AP, up to the highest argument it names at a fixed
  offset; up to the count, at most `max_args` (default 8), when it uses AP
  as an address, indexes it or offsets it by a variable (AMACRO's homing
  triggers), or with `home_args=TRUE`. A JSB routine that reads AP reads
  its caller's list, which the caller must have filled; `vmacro` says so.
  `home_args` and `quad_args` exclude each other. Code that writes AP is an
  error.
- **Calls.** `CALLS #n` sets `sp` to x18 rounded down to 16, lower by the
  arguments past the eighth, loads the n longwords pushed, sign-extended,
  into x0-x7 and the stack, pops them, sets x9 to n, and calls. `CALLG`
  does the same from a list in memory.
- **`RET`** loads the saved registers and returns.
- **Handlers.** `(FP)` and `0(FP)` as an operand mean the handler,
  `16(FP)`. Any other offset from FP at 0 or above is an error, as in
  AMACRO; negative offsets are the routine's locals.
- **Frame descriptors**: each CALL routine's goes in a psect of its own
  next to the code's (*The frame descriptor*); a frameless one has none.
- **64-bit.** DEC's pieces, under DEC's names: `quad_args`,
  `.ENABLE QUADWORD` and `/ENABLE=QUADWORD` for 64-bit address arithmetic,
  `$SETUP_CALL64`, `$PUSH_ARG64` and `$CALL64` with 8 register arguments,
  `EVAX_CALLG_64`, `$IS_32BITS`, `$IS_DESC64`, `$PUSH64`, `$POP64`, and the
  `EVAX_` built-ins in `vtools/docs/macro32.md`'s table, each compiled to
  the ARM64 instructions that do its job.

## BLISS-64

The default linkage, `CALL`, is this standard. `ACTUALCOUNT` and
`ACTUALPARAMETER` come from x9 and the arguments as *Argument lists*
says. `JSB` linkages are *JSB*, with registers named R0-R11 as in the
MACRO-32 table; the callee keeps what its declaration says. `ENABLE` uses
`16(FP)`. Every BLISS-64 routine with a frame has a descriptor.

## C and Fortran

A C or Fortran compiler conforms by:

- setting x9 on every call, and providing VMS's `va_count` from it;
- sign-extending 32-bit arguments and results;
- leaving x18 alone (clang's `-ffixed-x18`), and passing a static chain
  in x15, where GCC and LLVM would use x18;
- keeping frame pointers, and building this frame or describing its own
  in an unwind table;
- reserving `16(FP)` in a routine that calls `LIB$ESTABLISH`, as VAX C's
  `VAXC$ESTABLISH` needed compiler help.

A stock compiler does the last three with options and an unwind table,
but not the first two; until one is taught them, its code reaches VMS
routines that read their count or return longwords through jackets.
Fortran's VMS conventions sit on top: arguments by reference, `CHARACTER`
by descriptor, an omitted optional argument as 0 and the count, a
`CHARACTER` function's result through a hidden descriptor first.
