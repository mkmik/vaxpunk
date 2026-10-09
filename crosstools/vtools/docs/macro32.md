# vmacro: the MACRO-32 compiler

`vmacro` compiles VAX MACRO-32 source for ARM64 into an object module
(`docs/object-format.md`), which `vlink` links like any other. It does for
vaxpunk what the MACRO-32 compiler (AMACRO) did for OpenVMS Alpha: each VAX
instruction becomes a few native ones, with VAX registers, condition codes
and calls kept as the source expects them.

```
vmacro [/OBJECT=file | -o file] [/INCLUDE=dir | -I dir]... [/NOWARNINGS=NOTPIC | --nowarnings NOTPIC] [/ENABLE=QUADWORD | --enable QUADWORD] SOURCE
```

The options are vasm's (`docs/assembler.md`), and `/ENABLE=QUADWORD`, which
starts the module in quadword mode (*64-bit*). The object file defaults to
the source name with `.obj`.

```
vmacro hello.mar && vmacro -o consolio.obj crosstools/vtools/lib/consolio.mar
vmacro -o conputchar.obj crosstools/vtools/lib/conputchar.mar
vlink hello.obj consolio.obj conputchar.obj && vrun hello.exe
```

`crosstools/vtools/examples/macro32` has examples to try, with the commands in a
Justfile.

Status: integer instructions, the calling standard's `CALLS`, `CALLG` and
`RET`, `JSB` and `RSB`, `CASE`, bit fields, `MOVC3`, `MOVC5`, `CMPC3`, `CMPC5`,
`LOCC` and `SKPC`, `INSQUE` and
`REMQUE`, the privileged `MTPR`, `MFPR`, `HALT`, `CHMx`, `PROBEx` and `REI`, Alpha's
`CALL_PAL`, and AMACRO's 64-bit pieces: the `EVAX_` built-ins, `QUAD_ARGS`,
quadword mode and the 64-bit call macros. Not yet: floating point, packed decimal, the interlocked queue
instructions, the other string and privileged instructions, listings.

## How it works

vmacro is vasm with a MACRO-32 dialect. vasm reads the source: labels,
macros, `.IF` and repeat blocks, symbols and most directives work as in
`docs/assembler.md`. It passes each statement to vmacro, which translates VAX
instructions and MACRO-32's own directives into ARM64 assembly lines that vasm
then assembles. An error in the translation shows the MACRO-32 line and the
instruction it came from (`in MOVL, at hello.mar:12`).

A mnemonic that isn't VAX is left to vasm, so ARM64 instructions can be
mixed in: `svc #2` calls vrun's put. `vrun.mlb` works in MACRO-32 sources,
with the registers its macros use (x0 and x1 are R0 and R1). One that names
x2-x30, which vmacro uses itself or keeps VAX registers in, is a porting
message, once a routine: a built-in (*64-bit*) says the same in MACRO-32.
`.DISABLE FLAGGING` turns it off where raw ARM64 is meant, up to `.ENABLE
FLAGGING`.

Differences from vasm:

- Expressions are MACRO-32's: binary operators apply strictly left to right
  (`1+2*3` is 9), `<>` groups, `!` is OR, `\` XOR, `@` shifts (right if the
  count is negative), and `^C`, `^A/text/`, `^M<R2,R3>` complement, make ASCII
  values and register masks. `10.` is decimal. There is no `<<` or `>>`.
- An instruction after data (`.BYTE`, `.ASCIC`...) starts on the next
  longword, and labels on or just before it move with it. vasm itself
  requires `.ALIGN LONG`.

## Registers

| VAX | ARM64 |
| --- | --- |
| R0, R1 | x0, x1 |
| R2-R11 | x19-x28 |
| AP (R12) | the argument list at `32(FP)`, in x12 where the routine reads it |
| FP (R13) | x29 |
| SP (R14) | x18 |
| PC (R15) | no register; only in addressing modes as the VAX encodes them |

The map is the calling standard's
([DESIGN-0004](../../../docs/design/0004-calling-standard.md)): R2-R11 are in
AAPCS64's saved registers, and SP in x18, its platform register.

A register holds its longword sign-extended, as on Alpha: a longword
instruction that writes one leaves bits 63:32 equal to bit 31, and a byte or
word write changes only the low byte or word, as on the VAX, then
sign-extends from bit 31. `MOVAx` computes the address in longwords, so a
MACRO-32 image must lie below 2 GB; `vrun --base` must keep it there. A
quadword in registers is a pair: Rn low, Rn+1 high.

x2-x17 and x30 are the translation's: x0-x7 carry a call's arguments and x9
their count, x8-x11 and x13-x17 are scratch registers, x8 and x13 also
carry `CALLG`'s list and target, x2-x7 carry PAL call arguments, and x30 the
return address of `JSB`, as of any call. MACRO-32 code must not use them.

### The stack

VAX SP is x18, not ARM64's `sp`: VAX pushes longwords, and `sp` must stay
16-byte aligned at EL0 (vrun sets `SCTLR_EL1.SA0`, as seL4 and Linux do). Both
point into the same stack: before each call vmacro sets `sp` to x18 rounded
down to 16 bytes, so the callee's frame goes below what the caller pushed.

## Routines

Every routine is declared, as AMACRO requires
([amacro.md](amacro.md)): vmacro compiles only code in a declared routine,
which runs from its declaration to the next one. A label names the routine,
on the declaration's line or just before it:

```
; GETBYTE: R6 = a pointer. R0 = the byte there, R6 past it. Uses R2.
GETBYTE::
        .JSB_ENTRY  OUTPUT=<R6>, SCRATCH=<R2>
```

| Declaration | A routine |
| --- | --- |
| `.ENTRY name, mask` | called with `CALLS` or `CALLG`; defines `name` |
| `name: .CALL_ENTRY` | the same, without a mask |
| `name: .JSB_ENTRY` | called with `JSB` or `BSBx`; keeps every register it modifies but its outputs |
| `name: .JSB32_ENTRY` | the same, keeping none it isn't told to: for routines only MACRO-32 calls |
| `name: .EXCEPTION_ENTRY` | entered by the PAL, by `REI` or by a jump, never returning: interrupt and exception handlers, the code `REI` starts, error exits. Saves nothing |

Their parameters are AMACRO's: `OUTPUT=<R2,...>`, registers the routine
changes for its caller; `SCRATCH=<...>`, those it changes and its caller
doesn't care about; `PRESERVE=<...>`, those it always keeps, R0 and R1 too
in a JSB routine;
`INPUT=<...>`, documentation, but `INPUT=<AP>`: a JSB routine that reads
its caller's argument list (*Calls*). `.CALL_ENTRY` also takes `MAX_ARGS=n`,
`HOME_ARGS=TRUE|FALSE`, `QUAD_ARGS=TRUE|FALSE` and `LABEL=name`. A label
another routine branches to says so with `.GLOBAL_LABEL` after it.

### What a routine keeps

vmacro reads the module twice. The first pass surveys each routine: the
registers among R2-R11 it writes, and the JSB routines it calls. The second
compiles it, saving at entry and restoring on return all 64 bits of each
register the routine modifies, itself or through the JSB routines it calls,
but its `OUTPUT` and `SCRATCH`, as AMACRO did: a VAX routine's `PUSHR`
keeps only longwords, which would cut a 64-bit caller's values in half.

- A `.CALL_ENTRY` or `.ENTRY` routine saves those and the registers in its
  mask, in its frame. One it writes that the mask leaves out is a warning.
- A `.JSB_ENTRY` routine saves them in a save area it makes below both
  stacks at entry: the caller's `sp` and VAX SP, then the registers, then
  x30 if it calls. VAX SP and `sp` start below it, and `RSB` restores the
  registers and both stacks from it, so VAX SP must be back where the entry
  left it.
- A `.JSB32_ENTRY` routine saves only `PRESERVE`.

What a JSB routine modifies, for its callers: its `OUTPUT` and `SCRATCH`;
for a `.JSB32_ENTRY` one, everything it modifies. A JSB routine in another
module modifies all of R2-R11, unless `.CALL_LINKAGE name, ...` says what
it does, or `.DEFINE_LINKAGE name, ...` and `.USE_LINKAGE linkage_name=name`
before the `JSB`; `.USE_LINKAGE` with registers says it for a `JSB`
through an address. `vmacro::compile_modules`, which vms/build.rs
uses, compiles modules linked together and gives each the linkages of the
others' routines, from their declarations.

### Shared code

A routine may go to another routine's code, as VAX code does:

- from a JSB routine to a JSB routine's entry, a tail call: it restores
  what it saved, x30 too, then goes there;
- to a `.GLOBAL_LABEL` in another routine, if the two return alike: both
  CALL routines or both JSB routines, saving the same registers. vmacro
  makes routines that share code home the same arguments, read AP alike
  and keep x30 alike;
- from or to an `.EXCEPTION_ENTRY` routine, which saves nothing and never
  returns;
- after it loads SP from somewhere other than SP, a long jump, or to a
  label where code does, which leaves the routines on the stack.

A JSB routine that goes to its own entry goes on past its prologue, with
what it saved as it was. Anything else is an error that says what each
side restores. Code may run on into the next routine only from a JSB or
exception routine that saves nothing and has called nothing, whose x30 is
still its caller's, into one that saves nothing; anything else is an
error. So is a `JSB` to a local label, and a `JSB`, `CALLS` or `CALLG`
to a label in the module that isn't a routine's.

## Calls

`CALLS` and `CALLG` call as the calling standard says
([DESIGN-0004](../../../docs/design/0004-calling-standard.md), *Arguments*):
the first eight arguments in x0-x7, the rest on the stack from `sp` up, 8
bytes each, every longword sign-extended, and their count in x9.

- `CALLS #n, routine` sets `sp` to x18 rounded down to 16, lower by the
  arguments past the eighth, loads the n longwords the caller pushed,
  pops them, sets x9 and `bl`s the routine. `CALLS Rn, routine`, with a count known
  only when it runs, does the same in a loop.
- `CALLG arglist, routine` loads them from the list, as many as its count
  says. A list at address 0 passes none: a VAX routine that doesn't read
  AP never noticed it.
- A CALL routine builds the calling standard's frame on `sp`
  ([DESIGN-0004](../../../docs/design/0004-calling-standard.md), *Frames*):

  | Offset | Holds |
  | --- | --- |
  | 0 | the caller's FP, then LR: AAPCS64's frame record |
  | 16 | the condition handler, 0: `(FP)` and `0(FP)` mean it, so `MOVAB handler, (FP)` sets it |
  | 24 | the frame descriptor's address |
  | 32 | the argument list it homes, if any: the count, then a longword for each argument |
  | after it | the registers it saves, 8 bytes each: x18, the caller's SP, then those among R2-R11 |

  FP (x29) points at the frame, and SP (x18) starts at the frame, so
  locals made with `SUBL2 #n, SP` are at negative offsets from FP, as on
  the VAX. A routine with nothing to keep, which saves no registers, homes
  no arguments, calls nothing, leaves SP and FP alone and shares no code
  with another, is frameless, as the calling standard lets it be: no
  prologue, no descriptor, and `RET` is `ret`. A fault in it belongs to its
  caller's frame. Any other offset from FP at 0 or above is an error, as in
  AMACRO. Mask bits 12 and up (integer and decimal overflow traps) are
  ignored.
- **The argument list.** A routine that reads AP, or whose JSB routines
  in the module do, homes its arguments in its prologue, as AMACRO did: up
  to the highest it names at a fixed offset, `n(AP)` or `@n(AP)`; up to
  `MAX_ARGS`, 8 if not given, when it uses AP as a list (its address,
  indexed, offset by a variable or unaligned, or `CALLG (AP)`), or with
  `HOME_ARGS=TRUE`. The count is the caller's, but at most that. AP is x12,
  FP + 32, in a routine that reads it, and AP in a JSB routine is its
  caller's list; vmacro doesn't take code that writes AP.
- A JSB routine that reads AP, which no routine in the module calls, as a
  driver's FDT routine called through a table, is a warning: its callers
  must home what it reads, with `HOME_ARGS=TRUE` and enough `MAX_ARGS`.
  `INPUT=<AP>` says it means to.
- The descriptor, `$FDSCDEF`, says which registers the frame saves, where,
  its size and the routine's name. vmacro puts it in a psect of its own,
  the code's name with `_FDSC` after it, so that whoever may run the code
  may read it. vrun names a fault's frames from it, and `$UNWIND` restores
  what each frame saved by it.
- `RET` restores what the routine saved and returns. After code loads FP
  from elsewhere, `RET` returns from that frame, as its descriptor says,
  as a VAX `RET` reads the frame's mask.

vrun enters the transfer address as a call with one argument, the info
block: `4(AP)` in a MACRO-32 routine. Return a status in R0; vrun exits
with it.

`JSB` and `BSBx` set `sp` as a call does and are `bl`, and `RSB` is `ret`:
the return address is in x30, not on the VAX stack, as AMACRO made them
native calls. vmacro follows the VAX stack through each routine, and code
that pops, reads or pushes a return address there, or reads past what the
routine pushed, is an error, as is `JSB @(SP)+`.

## Console output

`crosstools/vtools/lib/consolio.mar` has VMS's console output routines, which its
bugcheck and init code, and some drivers, print with. They are JSB routines,
as on the VAX, and take no lock:

| Routine | Writes |
| --- | --- |
| `EXE$OUTCHAR` | the character in R0 |
| `EXE$OUTBLANK`, `EXE$OUTCRLF` | a space; CR and LF |
| `EXE$OUTHEX`, `EXE$OUTBYTE` | R1 as 8 hex digits; its low byte as 2 |
| `EXE$OUTZSTRING`, `EXE$OUTCSTRING` | the `.ASCIZ` (at most 255 characters) or `.ASCIC` string R1 points to; R1 ends past it |

The file says which registers each one uses. It is a module, not a macro
library: compile it and link it with the program, as the `hello` example
does. Its output goes one character at a time through `CON$PUTCHAR`, which
it doesn't define: under vrun, `crosstools/vtools/lib/conputchar.mar` writes with
vrun's put; the executive has its own, which writes `PR$_TXDB`.

```
MOVAB   MESSAGE, R1
JSB     G^EXE$OUTZSTRING
```

## Condition codes

ARM64's NZCV stand in for the VAX's NZVC. A compare, `ADDL`, `SUBL`, `INCL`,
`DECL`, `MNEGL`, `BICL`, `TST` and `BIT` set them directly. After a
subtraction or compare ARM64's C is the VAX's inverted (no borrow), and vmacro
remembers which, choosing the matching ARM64 condition for `BLSSU`, `BCS` and
the rest. Instructions whose result the VAX tests but ARM64 doesn't (moves,
logic, byte and word arithmetic) leave a test for the next conditional branch,
done only if one follows.

What doesn't carry over:

- A conditional branch must follow the instruction that set the codes, as
  written in the source; codes set on another path to a label aren't known.
- V and C are exact only for longword `ADD`, `SUB`, `INC`, `DEC` and
  `MNEG`, and C for `CMP`, whose V is ARM64's: set on a signed overflow,
  where the VAX clears it. Other instructions clear them rather than leave
  them as the VAX would.
- The signed branches (`BLSS`, `BGEQ`, `BGTR`, `BLEQ`) test N xor V, as
  ARM64's do, so after an arithmetic instruction that overflows they go
  the other way from the VAX's, which test N.
- No arithmetic traps: overflow and divide by zero don't fault. A divide by
  zero gives 0.

## Instructions

| Group | Instructions |
| --- | --- |
| Move | `MOVx`, `CLRx`, `MCOMx`, `MNEGx`, `MOVZBW`, `MOVZBL`, `MOVZWL`, `CVTBW`, `CVTBL`, `CVTWL`, `CVTWB`, `CVTLB`, `CVTLW`, `MOVAx`, `PUSHAx`, `PUSHL`, `PUSHR`, `POPR` |
| Arithmetic and logic | `ADDx2/3`, `SUBx2/3`, `MULx2/3`, `DIVx2/3`, `BISx2/3`, `BICx2/3`, `XORx2/3`, `INCx`, `DECx`, `CMPx`, `TSTx`, `BITx`, `ASHL`, `ASHQ`, `ROTL`, `EMUL`, `EDIV` |
| Branch | `BRB`, `BRW`, `Bcc` (all 16), `BLBS`, `BLBC`, `BBS`, `BBC`, `BBSS`, `BBSC`, `BBCS`, `BBCC`, `JMP`, `CASEB/W/L`, `ACBB/W/L`, `AOBLSS`, `AOBLEQ`, `SOBGTR`, `SOBGEQ` |
| Call | `CALLS`, `CALLG`, `RET`, `JSB`, `BSBB`, `BSBW`, `RSB` |
| Field and string | `EXTV`, `EXTZV`, `INSV`, `MOVC3`, `MOVC5`, `CMPC3`, `CMPC5`, `LOCC`, `SKPC` |
| Queue | `INSQUE`, `REMQUE` |
| Privileged | `MTPR`, `MFPR`, `HALT`, `CHMK`, `CHME`, `CHMS`, `CHMU`, `PROBER`, `PROBEW`, `REI`, `CALL_PAL`: PAL calls, see *Privileged instructions* |
| 64-bit | AMACRO's `EVAX_` built-ins, `EVAX_CALLG_64`: see *64-bit* |
| Other | `NOP`, `BPT` (`brk`) |

`x` is B, W, L, or Q where the VAX has it. Every addressing mode works:
register, `(Rn)`, `(Rn)+`, `-(Rn)`, `@(Rn)+`, `d(Rn)` and `@d(Rn)` with or
without `B^`/`W^`/`L^`, `#n` with `S^`/`I^`, `@#address`, `address` and
`@address` (`G^` too), and `[Rx]` indexing any memory mode. Operands are
evaluated left to right with their side effects, as on the VAX.

A jump or call to `address` or `G^address` is `b` or `bl`, which reach
±128 MB. A `G^` target may be in another image, as `SYS$name` in the
executive or `LIB$name` in a shareable image is: there the linker puts a
veneer between the call and its target (`docs/linker.md`). A jump or call to
a constant address, or through any other operand, goes through a register.

Limits:

- `CASE`'s limit must be an immediate. The table of `.WORD` displacements
  follows the instruction as usual.
- A bit field's size must be a constant, at most 32. A field in a register
  must fit in it. A field in memory is read and written as the 8 bytes
  around it: not atomically, and a field ending less than 8 bytes before an
  unmapped page faults.
- `ASHL` and `ASHQ` with a count in a register wrap counts of 32 (64) and
  more instead of clearing the result.
- `MOVC5` copies forwards only; `MOVC3` handles any overlap. Both copy a byte
  at a time, and `CMPC3`, `CMPC5`, `LOCC` and `SKPC` look at one.
- `PUSHR` and `POPR` take a constant mask and can't save SP or PC.
- `INSQUE` and `REMQUE` aren't interlocked, and the queue's links are
  longwords, so it must lie below 4 GB. They set Z as the VAX does: the
  entry inserted is the only one, the queue removed from is now empty;
  `REMQUE` sets V when the queue was empty and there was nothing to remove.

## Privileged instructions

The executive's privileged instructions are calls to the PAL below it, as
AMACRO made them `CALL_PAL`s on Alpha. vmacro compiles each into `svc #0`
with the PAL function code in x7, the argument in x0 and the result back in
x0, keeping R0 in a scratch register around it. The interface and its
function codes are in
[DESIGN-0001](../../../docs/design/0001-pal-interface.md).

| Instruction | PAL call |
| --- | --- |
| `MTPR src, #PR$_IPL`, `MFPR #PR$_IPL, dst` | `MTPR_IPL`, `MFPR_IPL` |
| `MFPR #PR$_PCBB, dst` | `MFPR_PCBB` |
| `MTPR src, #PR$_SCBB`, `MFPR #PR$_SCBB, dst` | `MTPR_SCBB`, `MFPR_SCBB` |
| `MTPR src, #PR$_SIRR`, `MFPR #PR$_SISR, dst` | `MTPR_SIRR`, `MFPR_SISR` |
| `MTPR src, #PR$_TXDB` | `MTPR_TXDB`, a console character |
| `MTPR src, #PR$_RXCS`, `MFPR #PR$_RXCS, dst` | `MTPR_RXCS`, `MFPR_RXCS`: the console receive status, a character waiting and its interrupt enable |
| `MFPR #PR$_RXDB, dst` | `MFPR_RXDB`, the console character received |
| `MTPR src, #PR$_DOORBELL` | `MTPR_DOORBELL`, rings port src's doorbell |
| `CHMK #code`, `CHME`, `CHMS`, `CHMU` | `CHMK`, `CHME`, `CHMS`, `CHMU`: the code goes in x7's bits 31:16, so the arguments in x0-x6 reach the handler, and R0 and R1 come back with what the service left in the frame |
| `PROBER mode, len, base`, `PROBEW` | `PROBER`, `PROBEW`: base, len and mode in x0-x2, R0 and R1 kept; Z is set if the mode may not read (write) the first and last byte, as on the VAX |
| `REI` | `REI`: resumes at the PC in the frame on the stack, with every register from it |
| `HALT` | `HALT` |
| `CALL_PAL #code` | any PAL call, as on Alpha: arguments in R0-R5, which go to x0-x5, the result in R0 |

`CALL_PAL` is for the calls the VAX has no instruction for, `SWPCTX`,
`WTINT`, `WRPTE`, `DELCTX`, `READLBLK`, `WRITELBLK`, `GETENTROPY` and `RD_PS`, whose
codes `$PALDEF` names.
The code must be a constant.

The processor register must be a constant, as in AMACRO. `$PRDEF` in
`crosstools/vtools/lib/lib.mlb` defines the `PR$_` names:

```
        .LIBRARY "lib.mlb"
        $PRDEF
        MTPR    R0, #PR$_TXDB
```

`MTPR` and `MFPR` set N and Z from the value and clear V and C. The VAX
leaves C alone. `CHMx` and `CALL_PAL` set them from R0, `PROBEx` from the
PAL's result.

`lib.mlb` also has the executive's structures (`$PCBDEF`, `$PTEDEF`,
`$RPBDEF`...) and VMS's IPL macros, `SETIPL`, `DSBINT`, `ENBINT` and
`SOFTINT`. `crosstools/vtools/lib/starlet.mlb` is for programs that call system
services: `$SSDEF`, `$PRTDEF`, and the `$name_S` macros, which push the
arguments and call `SYS$name`.

vrun runs programs in user mode, where these instructions don't work: each
one stops the run with `%VRUN-F-OPCDEC`.

## 64-bit

MACRO-32 stays a 32-bit language, and 64-bit code says so, with AMACRO's
pieces ([amacro.md](amacro.md), *64-bit*), which vmacro compiles to the
ARM64 that does each one's job.

**Built-ins.** Each takes any VAX operand mode, as a quadword: a register
operand is all 64 bits of its x register, not a pair, a memory one the
quadword there, a literal a longword, sign-extended, as AMACRO evaluates
expressions, or 64 bits in quadword mode. A load or store built-in's first operand
is a register. A register a built-in leaves with a 64-bit value must be
back in sign-extended longword form before a VAX instruction reads it. The
condition codes are unpredictable after a built-in, as on Alpha, which has
none.

| Built-in | Does |
| --- | --- |
| `EVAX_SEXTB`, `SEXTW`, `SEXTL src, dst` | dst = src's low byte, word or longword, sign-extended: all 64 bits of a register, a byte, word or longword in memory |
| `EVAX_LDQ Rn, src`, `EVAX_STQ Rn, dst` | Rn's 64 bits from or to an operand |
| `EVAX_LDBU`, `LDWU Rn, src` | Rn = a byte or word from memory, zero-extended |
| `EVAX_STB`, `STW Rn, dst` | Rn's low byte or word to memory |
| `EVAX_LDAQ Rn, src` | Rn = src's address, 64 bits |
| `EVAX_LDQU`, `STQU Rn, mem` | Rn from or to the quadword at mem's address rounded down to 8 |
| `EVAX_LDLL`, `LDQL Rn, mem` | a longword (sign-extended) or quadword load that starts an exclusive access (`ldxr`) |
| `EVAX_STLC`, `STQC Rn, mem` | the store that ends it (`stxr`): Rn = 1 if it did, else 0 |
| `EVAX_ADDQ`, `SUBQ`, `MULQ`, `UMULH a, b, c` | c = a + b, a - b, a × b, the high 64 bits of a × b unsigned |
| `EVAX_AND`, `OR`, `XOR`, `BIC`, `ORNOT`, `EQV a, b, c` | c = a & b, a \| b, a ^ b, a & ~b, a \| ~b, a ^ ~b |
| `EVAX_SLL`, `SRL`, `SRA a, b, c` | c = a shifted by b's low 6 bits: left, right, right with its sign |
| `EVAX_ZAP`, `ZAPNOT a, mask, c` | c = a with the bytes mask's bits 0-7 name cleared, or kept |
| `EVAX_CMPEQ`, `CMPLT`, `CMPLE`, `CMPULT`, `CMPULE a, b, c` | c = 1 if a = b, a < b, a ≤ b signed, a < b, a ≤ b unsigned, else 0 |
| `EVAX_BEQ`, `BLT`, `BNE src, label` | branch if src's 64 bits are 0, negative, not 0 |
| `EVAX_CMOVEQ`, `NE`, `LT`, `LE`, `GT`, `GE`, `LBC`, `LBS a, b, c` | c = b if a is 0, not 0, < 0, ≤ 0, > 0, ≥ 0, has its low bit clear, set |
| `EVAX_MB` | a memory barrier (`dmb sy`) |
| `EVAX_MTPR_x src`, `EVAX_MFPR_x` | the PAL call for processor register x (IPL, SCBB, SIRR, SISR, PCBB, TXDB, RXCS, RXDB, DOORBELL); R0 = its result |

The rest of Alpha's (byte manipulation, `EVAX_TRAPB`, `RPCC`, the FPCR, the
PAL calls vaxpunk has no meaning for) is an error that names the built-in.

**Arguments.** `.CALL_ENTRY QUAD_ARGS=TRUE` says the routine's arguments
are quadwords: it homes them as quadwords, the count too, `4n(AP)` still
naming argument n, and a quadword instruction reads all of one, as
`MOVQ 4(AP), 8(R2)` stores the whole of argument 1; `@n(AP)` loads a 64-bit
pointer. `CALLG (AP)` passes the list on as quadwords, and indexing AP by
bytes or words is an error. It excludes `HOME_ARGS`, and a JSB routine
can't read such a list. `EVAX_CALLG_64 list, routine` is `CALLG` with a list
of quadwords, a quadword count first.

**Calls with 64-bit arguments.** `$SETUP_CALL64 n` starts a call with n
arguments, `$PUSH_ARG64 op` puts each, the last first, as a quadword, and
`$CALL64 routine` calls with them in x0-x7 and on the stack, their count in
x9; it is an error if the pushes don't match the count. vmacro does them
itself, since they put arguments where MACRO-32 can't name them: the
arguments wait in slots below both stacks, so any operand may be pushed,
R0 and R1 too, but nothing between `$SETUP_CALL64` and `$CALL64` may push,
pop, call or return. `INLINE=` is accepted and changes nothing.

**Address arithmetic.** In quadword mode, from `.ENABLE QUADWORD` or
`/ENABLE=QUADWORD` to `.DISABLE QUADWORD`, a built-in's literal is 64 bits,
and `MOVAx` to a register writes the whole 64-bit address; otherwise it is a longword, sign-extended, as AMACRO
computed it: `MOVAL (R1)[R0], R2` with R1 = `7FFFFFFF` and R0 = 1 gives
`FFFFFFFF.80000003`, or `00000000.80000003` in quadword mode. Memory
operands' addresses are 64-bit either way: `MOVL 4(R1), R0` reads the
longword at R1 + 4 whatever R1 holds.

**Tests and stacks.** `$IS_32BITS q, leq[, gtr]` branches to leq if the
quadword q is a sign-extended longword, else to gtr. `$IS_DESC64 desc,
label[, SIZE=LONG|QUAD]` branches if the descriptor whose address desc
holds, a longword or a quadword, is in 64-bit form: MBO 1 and MBMO -1
(`$DSCDEF`'s `DSC64$`; `$ILEDEF` has the 64-bit item lists). `$PUSH64 Rn`
and `$POP64 Rn` keep all 64 bits of a register on the VAX stack.

## Directives

vasm's directives work (`.TITLE`, `.IDENT`, `.PSECT`, data, `.ALIGN`, `.END`,
macros, conditionals...), with these changes:

| Directive | In vmacro |
| --- | --- |
| `.ENTRY name, mask`, `.CALL_ENTRY`, `.JSB_ENTRY`, `.JSB32_ENTRY`, `.EXCEPTION_ENTRY`, `.GLOBAL_LABEL` | routines: see *Routines* |
| `.CALL_LINKAGE`, `.DEFINE_LINKAGE`, `.USE_LINKAGE` | what JSB routines elsewhere modify: see *What a routine keeps* |
| `.WARN text` | a warning |
| `.ADDRESS` | a longword address, as on the VAX (vasm's is a quadword) |
| `.BLKA` | longwords |
| `.EXTRN` | `.EXTERNAL` |
| `.SIGNED_BYTE`, `.SIGNED_WORD` | `.BYTE`, `.WORD` |
| `.PSECT name, EXE, ...` | also `NOWRT`: ARM64 code can't be writable. `USR` and `LIB` are dropped |
| `.ERROR text` | an error |
| `.ENABLE QUADWORD`, `.DISABLE QUADWORD` | address arithmetic in 64 bits, or 32: see *64-bit* |
| `.ENABLE FLAGGING`, `.DISABLE FLAGGING` | porting messages for raw ARM64, or none |
| listing directives (`.SBTTL`, `.PAGE`, `.LIST`, `.SHOW`, the other `.ENABLE` and `.DISABLE` options, `.DEFAULT`, `.PRINT`...) | ignored |

A psect for code needs `EXE`: with vasm's defaults a psect such as `.PSECT
CODE` is data, and running it faults. `$CODE$` has the right attributes.

An operand vmacro can't evaluate yet, such as `#label` or a displacement
defined further down, comes from a longword in `$LINK$`, or a quadword for a
built-in's literal in quadword mode, which the loader
fixes up if the image moves. A constant known at that point is built into the
instructions.

## Tests

`crosstools/vtools/examples/macro32/` holds examples with a Justfile that runs them
with the vtools commands, and `crosstools/vtools/tests/macro32/` programs that check
themselves. `cargo test -p vrun --test programs macro32` compiles, links and
runs both under vrun, at the link base and moved, against their expected
output, linking each against the modules in `crosstools/vtools/lib`.
`crates/vmacro/tests/errors.rs` checks the errors.
