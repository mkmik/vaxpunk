# vmacro: the MACRO-32 compiler

`vmacro` compiles VAX MACRO-32 source for ARM64 into an object module
(`docs/object-format.md`), which `vlink` links like any other. It does for
vaxpunk what the MACRO-32 compiler (AMACRO) did for OpenVMS Alpha: each VAX
instruction becomes a few native ones, with VAX registers, condition codes
and calls kept as the source expects them.

```
vmacro [/OBJECT=file | -o file] [/INCLUDE=dir | -I dir]... [/NOWARNINGS=NOTPIC | --nowarnings NOTPIC] SOURCE
```

The options are vasm's (`docs/assembler.md`). The object file defaults to the
source name with `.obj`.

```
vmacro hello.mar && vmacro -o consolio.obj vtools/lib/consolio.mar
vlink hello.obj consolio.obj && vrun hello.exe
```

`vtools/examples/macro32` has examples to try, with the commands in a
Justfile.

Status: integer instructions, the calling standard's `CALLS`, `CALLG` and
`RET`, `JSB` and `RSB`, `CASE`, bit fields, `MOVC3` and `MOVC5`, `INSQUE` and
`REMQUE`, the privileged `MTPR`, `MFPR`, `HALT`, `CHMx`, `PROBEx` and `REI`, and Alpha's
`CALL_PAL`. Not yet: floating point, packed decimal, the interlocked queue
instructions, the other string and privileged instructions, `.CALL_ENTRY`,
listings.

## How it works

vmacro is vasm with a MACRO-32 dialect. vasm reads the source: labels,
macros, `.IF` and repeat blocks, symbols and most directives work as in
`docs/assembler.md`. It passes each statement to vmacro, which translates VAX
instructions and MACRO-32's own directives into ARM64 assembly lines that vasm
then assembles. An error in the translation shows the MACRO-32 line and the
instruction it came from (`in MOVL, at hello.mar:12`).

A mnemonic that isn't VAX is left to vasm, so ARM64 instructions can be
mixed in, like AMACRO's `EVAX_` built-ins: `svc #2` calls vrun's put. `vrun.mlb`
works in MACRO-32 sources, with the registers its macros use (x0 and x1 are R0
and R1).

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
| AP (R12) | x12 |
| FP (R13) | x29 |
| SP (R14) | x18 |
| PC (R15) | no register; only in addressing modes as the VAX encodes them |

The map is the calling standard's
([DESIGN-0004](../../docs/design/0004-calling-standard.md)): R2-R11 are in
AAPCS64's saved registers, and SP in x18, its platform register.

A register holds its longword sign-extended, as on Alpha: a longword
instruction that writes one leaves bits 63:32 equal to bit 31, and a byte or
word write changes only the low byte or word, as on the VAX, then
sign-extends from bit 31. `MOVAx` computes the address in longwords, so a
MACRO-32 image must lie below 2 GB; `vrun --base` must keep it there. A
quadword in registers is a pair: Rn low, Rn+1 high.

x2-x17 are the translation's: x8-x11 and x13-x17 are scratch registers, x13
also passes the argument list pointer in a call, and x2-x7 carry PAL call
arguments. MACRO-32 code must not use them.

### The stack

VAX SP is x18, not ARM64's `sp`: VAX pushes longwords, and `sp` must stay
16-byte aligned at EL0 (vrun sets `SCTLR_EL1.SA0`, as seL4 and Linux do). Both
point into the same stack: before each call vmacro sets `sp` to x18 rounded
down to 16 bytes, so the callee's frame goes below what the caller pushed.

## Calls

`CALLS` and `CALLG` keep the VAX calling standard's shape: an argument list of
longwords, a count first, that AP points to in the called routine. Provisional
until vaxpunk's calling standard exists.

- `CALLS #n, routine` pushes the count on the stack above the arguments the
  caller pushed, points x13 at it, aligns `sp` and `bl`s the routine. On
  return it pops the list. `CALLG arglist, routine` passes `arglist` in x13 and
  pops nothing.
- `.ENTRY name, ^M<R2,...>` defines a global `name` and builds a frame on `sp`:

  | Offset | Holds |
  | --- | --- |
  | 0 | condition handler, 0 (`MOVAB handler, (FP)` sets it) |
  | 8 | the caller's AP |
  | 16 | the caller's FP, then LR |
  | 32 | the caller's SP (x18) |
  | 40 | the entry mask's bits 11:0, which say what follows, for `$UNWIND` |
  | 48 | the registers in the entry mask, 8 bytes each |

  FP (x29) points at the frame, AP (x12) at the argument list, and SP (x18)
  starts at the frame, so locals made with `SUBL2 #n, SP` are at negative
  offsets from FP, as on the VAX. Mask bits 12 and up (integer and decimal
  overflow traps) are ignored.
- `RET` restores what `.ENTRY` saved and returns. It belongs to the last
  `.ENTRY` before it in the source.

A routine called from outside MACRO-32 gets no argument list: vrun enters the
transfer address with x13, and so AP, 0. Return a status in R0; vrun exits
with it.

`JSB` and `BSBx` push an 8-byte return address on the stack and jump; `RSB`
pops it and jumps to it, so a JSB routine needs no declaration
(`.JSB_ENTRY` is accepted and ignored). Code that pops or changes the return
address as a longword won't work.

## Console output

`vtools/lib/consolio.mar` has VMS's console output routines, which its
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
it doesn't define: under vrun, `vtools/lib/conputchar.mar` writes with
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
- V and C are exact only for longword `ADD`, `SUB`, `INC`, `DEC`, `CMP` and
  `MNEG`. Other
  instructions clear them rather than leave them as the VAX would.
- No arithmetic traps: overflow and divide by zero don't fault. A divide by
  zero gives 0.

## Instructions

| Group | Instructions |
| --- | --- |
| Move | `MOVx`, `CLRx`, `MCOMx`, `MNEGx`, `MOVZBW`, `MOVZBL`, `MOVZWL`, `CVTBW`, `CVTBL`, `CVTWL`, `CVTWB`, `CVTLB`, `CVTLW`, `MOVAx`, `PUSHAx`, `PUSHL`, `PUSHR`, `POPR` |
| Arithmetic and logic | `ADDx2/3`, `SUBx2/3`, `MULx2/3`, `DIVx2/3`, `BISx2/3`, `BICx2/3`, `XORx2/3`, `INCx`, `DECx`, `CMPx`, `TSTx`, `BITx`, `ASHL`, `ASHQ`, `ROTL`, `EMUL`, `EDIV` |
| Branch | `BRB`, `BRW`, `Bcc` (all 16), `BLBS`, `BLBC`, `BBS`, `BBC`, `BBSS`, `BBSC`, `BBCS`, `BBCC`, `JMP`, `CASEB/W/L`, `ACBB/W/L`, `AOBLSS`, `AOBLEQ`, `SOBGTR`, `SOBGEQ` |
| Call | `CALLS`, `CALLG`, `RET`, `JSB`, `BSBB`, `BSBW`, `RSB` |
| Field and string | `EXTV`, `EXTZV`, `INSV`, `MOVC3`, `MOVC5` |
| Queue | `INSQUE`, `REMQUE` |
| Privileged | `MTPR`, `MFPR`, `HALT`, `CHMK`, `CHME`, `CHMS`, `CHMU`, `PROBER`, `PROBEW`, `REI`, `CALL_PAL`: PAL calls, see *Privileged instructions* |
| 64-bit | `EVAX_LDQ Rn, src`, `EVAX_STQ Rn, dst`: all 64 bits of Rn, as AMACRO's built-ins |
| Other | `NOP`, `BPT` (`brk`) |

`x` is B, W, L, or Q where the VAX has it. Every addressing mode works:
register, `(Rn)`, `(Rn)+`, `-(Rn)`, `@(Rn)+`, `d(Rn)` and `@d(Rn)` with or
without `B^`/`W^`/`L^`, `#n` with `S^`/`I^`, `@#address`, `address` and
`@address` (`G^` too), and `[Rx]` indexing any memory mode. Operands are
evaluated left to right with their side effects, as on the VAX.

A jump or call to `address` is `b` or `bl`, which reach ±128 MB: the image
it is in. One to `G^address`, which may be in another image, as `SYS$name`
in the executive is, takes the address with `adrp` and `add`, ±4 GB, and
goes through a register.

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
  at a time.
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
[DESIGN-0001](../../docs/design/0001-pal-interface.md).

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
| `CHMK #code`, `CHME`, `CHMS`, `CHMU` | `CHMK`, `CHME`, `CHMS`, `CHMU`: the code goes in R0, and R0 and R1 come back with what the service left in the frame |
| `PROBER mode, len, base`, `PROBEW` | `PROBER`, `PROBEW`: base, len and mode in x0-x2, R0 and R1 kept; Z is set if the mode may not read (write) the first and last byte, as on the VAX |
| `REI` | `REI`: resumes at the PC in the frame on the stack, with every register from it |
| `HALT` | `HALT` |
| `CALL_PAL #code` | any PAL call, as on Alpha: arguments in R0-R5, which go to x0-x5, the result in R0 |

`CALL_PAL` is for the calls the VAX has no instruction for, `SWPCTX`,
`WTINT`, `WRPTE`, `DELCTX` and `READLBLK`, whose codes `$PALDEF` names.
The code must be a constant.

The processor register must be a constant, as in AMACRO. `$PRDEF` in
`vtools/lib/lib.mlb` defines the `PR$_` names:

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
`SOFTINT`. `vtools/lib/starlet.mlb` is for programs that call system
services: `$SSDEF`, `$PRTDEF`, and the `$name_S` macros, which push the
arguments and call `SYS$name`.

vrun runs programs in user mode, where these instructions don't work: each
one stops the run with `%VRUN-F-OPCDEC`.

## Directives

vasm's directives work (`.TITLE`, `.IDENT`, `.PSECT`, data, `.ALIGN`, `.END`,
macros, conditionals...), with these changes:

| Directive | In vmacro |
| --- | --- |
| `.ENTRY name, mask` | a routine: see *Calls* |
| `.ADDRESS` | a longword address, as on the VAX (vasm's is a quadword) |
| `.BLKA` | longwords |
| `.EXTRN` | `.EXTERNAL` |
| `.SIGNED_BYTE`, `.SIGNED_WORD` | `.BYTE`, `.WORD` |
| `.PSECT name, EXE, ...` | also `NOWRT`: ARM64 code can't be writable. `USR` and `LIB` are dropped |
| `.ERROR text` | an error |
| `.JSB_ENTRY`, listing directives (`.SBTTL`, `.PAGE`, `.LIST`, `.SHOW`, `.ENABLE`, `.DISABLE`, `.DEFAULT`, `.PRINT`, `.WARN`...) | ignored |

A psect for code needs `EXE`: with vasm's defaults a psect such as `.PSECT
CODE` is data, and running it faults. `$CODE$` has the right attributes.

An operand vmacro can't evaluate yet, such as `#label` or a displacement
defined further down, comes from a longword in `$LINK$`, which the loader
fixes up if the image moves. A constant known at that point is built into the
instructions.

## Tests

`vtools/examples/macro32/` holds examples with a Justfile that runs them
with the vtools commands, and `vtools/tests/macro32/` programs that check
themselves. `cargo test -p vrun --test programs macro32` compiles, links and
runs both under vrun, at the link base and moved, against their expected
output, linking each against the modules in `vtools/lib`.
`crates/vmacro/tests/errors.rs` checks the errors.
