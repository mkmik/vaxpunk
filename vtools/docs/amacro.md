# What DEC's MACRO-32 compiler did, and what vmacro takes from it

Oct 6, 2026 · @Marko Mikulicic

When OpenVMS moved to Alpha, DEC turned MACRO-32 from the VAX's assembler
into a compiled language: the MACRO-32 Compiler for OpenVMS Alpha
(AMACRO), later retargeted to Itanium (IMACRO) and x86-64 (XMACRO). V7.0
added the constructs that let MACRO-32 handle 64-bit addresses. `vmacro`
does for ARM64 what AMACRO did for Alpha, so this file collects what the
primary sources say about it, as the reference
[DESIGN-0004](../../docs/design/0004-calling-standard.md) and
[PRD-0005](../../docs/prd/0005-macro32-on-the-calling-standard.md) build
on. The last section says what vmacro takes, changes or leaves.

Sources, cited by tag with their own section and page numbers:

| Tag | Document |
| --- | --- |
| [MCG] | *VSI OpenVMS MACRO Compiler Porting and User's Guide*, Alpha V8.4-2L1 / IA-64 V8.4-1H1 edition, 2026 (docs.vmssoftware.com). Same content as the April 2019 edition DO-DMCPUG-01A. |
| [CS] | *VSI OpenVMS Calling Standard*, x86-64 V9.2-2 / IA-64 V8.4-1H1 / Alpha V8.4-2L1, 2026 (docs.vmssoftware.com). Printed page = PDF page − 16. |
| [64G] | *OpenVMS Alpha Guide to 64-Bit Addressing and VLM Features*, V7.2-1, June 1999 (the V7.1 edition and the V8.3 MACRO guide's chapter 5 agree). |
| [PC2] | *VSI OpenVMS Programming Concepts Manual Vol. II*, x86-64 V9.0, April 2020, §2.1.4. |
| [HELP] | The MACRO help library (wasd.vsm.com.au helpgate), I64 era. |

The manuals are HP and VSI copyright: this file is notes in our own words
with short quotations, never copies.

## Registers

**Alpha** ([MCG] §2.1). VAX R0-R12 are Alpha R0-R12; AP, FP and SP become
the Alpha registers with the same jobs. Longword instructions use the low
32 bits, and "the compiler maintains a sign-extended 64-bit form of this
value in the register". R13 and up written in the source mean the Alpha
registers themselves; the compiler may use R13-R15 as temporaries and
saves them if it does. This fit because the OpenVMS Alpha calling standard
made R2-R15 saved registers ([CS] §3.1: "If a standard-conforming procedure
modifies one of these registers, it must save and restore it").

**Itanium** ([MCG] §2.2, Table 2.1). The compiler maps source registers
onto Itanium's to fit Intel's conventions: R0 to r8 and R1 to r9, the
return registers, so `MOVL #SS$_NORMAL, R0` returns its value; R2 to r28,
R3-R7 to r3-r7, R8 to r26, R9 to r27, R10-R11 to r10-r11, and so on. "The
compiler does not provide any syntax for accessing Itanium registers
directly." Code that named Alpha's argument registers R16-R21 "is not
portable to OpenVMS I64"; use `PUSHL` and `n(AP)`.

**x86-64** ([PC2] §2.1.4.7, [CS] §5.1.8). x86-64 has too few registers,
so XMACRO keeps 32 "pseudo-registers" R0-R31 "as a per-thread vector of
quadwords in memory", well defined only at calls and returns, reachable
from other languages through `LIB$GET_ALPHA_REG_VECTOR`. VSI calls their
use outside legacy code deprecated.

## Declaring routines

Declaring routines is mandatory ([MCG] §2.3): "The compiler generates code
only for source instructions that are part of a declared routine." Every
target of `CALLS`, `CALLG`, `JSB`, `BSBW` and `BSBB` is declared, as is a
target of a cross-module or indirect branch and any label whose address is
stored. A global label that isn't an entry point says so with
`.GLOBAL_LABEL`. `.ENTRY` is accepted and becomes `.CALL_ENTRY`.

**Register preservation** is automatic and 64-bit ([MCG] App. B):
`.CALL_ENTRY` and `.JSB_ENTRY` "save and restore the full 64 bits of any
registers (except R0 and R1) that are modified by the routine and are not
declared as scratch or output". A register the routine doesn't modify
isn't saved. The reason is that `PUSHL` and `PUSHR` keep only longwords
(§2.4.2), and 64-bit callers would lose upper halves. Parameters, each a
register set like `<R2,R3>`:

| Parameter | `.CALL_ENTRY` | `.JSB_ENTRY` | Meaning |
| --- | --- | --- | --- |
| `max_args=n` | yes | | longwords to reserve if the argument list is homed |
| `home_args=TRUE\|FALSE` | yes | | force or forbid homing |
| `quad_args=TRUE\|FALSE` | yes | | the routine reads quadword arguments (*64-bit*) |
| `input` | yes | yes | documentation; keeps the compiler off those registers as temporaries |
| `output` | yes | yes | modified and live at exit: not restored |
| `scratch` | yes | yes | modified and dead at exit: not restored |
| `preserve` | yes | yes | always saved and restored in full, R0 and R1 too; overrides `output` and `scratch` |
| `label=name` | yes | | the name to use with `.ENTRY` when assembling for the VAX |

`.JSB32_ENTRY` takes the same register sets but "does not preserve any
VAX register values (R2 through R12) unless the PRESERVE parameter is
specified". It is for routines only 32-bit MACRO-32 calls, never for code
other languages call, AST routines or condition handlers. DEC's pattern is
a barrier: the outer routine is `.JSB_ENTRY` or `.CALL_ENTRY` and keeps
64 bits; the routines behind it are `.JSB32_ENTRY` (§2.5.2).

On Alpha a JSB routine got a null-frame procedure descriptor: it runs in
its caller's context and is never "current" ([CS] §3.4.7). Writing `0(FP)`
in one is illegal (§2.5.4).

`.EXCEPTION_ENTRY` (Alpha) declares an interrupt or exception service
routine, entered with R2-R7, PC and PS pushed and left with `REI`.
Condition handlers are ordinary `.CALL_ENTRY` routines (§2.8).

On Itanium, where the mapping put some VAX registers on scratch hardware
registers, `.CALL_LINKAGE`, `.DEFINE_LINKAGE` and `.USE_LINKAGE` told the
compiler what a callee keeps and returns, so it could save registers
around the call: needed for callees that return in registers other than
R0 and R1, and for JSB targets written in other languages (§1.4).

**Shared code** ([MCG] §2.7). Branching between routines is allowed but
flagged. Routines that share code with different register declarations
get conditional restores, driven by a mask saved at entry. A CALL routine
that reaches `RSB` is an error.

## Arguments

The compiler turns `n(AP)` into references to the arguments where the
calling standard put them, in registers or on the stack ([MCG] §2.4). It
homes them, copying them into a VAX-style longword list in the frame's
fixed temporaries, addressed from FP, only when it must: when AP or an
address based on it is stored or passed, indexed or offset by a variable,
or used at an unaligned offset like `6(AP)` (§2.4.1). `home_args=TRUE`
forces it. A JSB routine that reads AP is assumed to read the list homed by
"the last .CALL_ENTRY routine", with a message, which is what
`home_args=TRUE` on the caller is for. A routine that modifies AP gets
every use of AP turned into R12, and walking the argument list that way
"is not supported" (§2.4.3).

| | Arguments in registers | Argument information |
| --- | --- | --- |
| Alpha | 6, R16-R21 or F16-F21 by slot | R25 ([CS] §3.6.1): count <7:0>, six 3-bit register types <25:8> |
| Itanium | 8, out0-out7 | R25 ([CS] §4.7.5.3): count <7:0>, eight 3-bit types <31:8>, <63:32> zero |
| x86-64 | 6, System V | %rax ([CS] Table 5.13): XMM count <7:0>, argument slots <15:8>, offset to an argument information block <47:16> |

On x86-64 "it is not possible to create a register home on the stack that
is contiguous with the incoming memory arguments" ([CS] §5.7.5.2): a callee
that needs a list builds one and copies into it. AAPCS64 is the same.

`CALLS` with a constant count turns the pushes before it into register
loads. A variable `CALLS` and `CALLG` go through a run-time routine that
unpacks the list first (§2.3.5).

## Idioms the compiler flags

From [MCG] §3.1-3.3, as errors unless marked:

- positive offsets from FP other than `0(FP)`, and references into the
  caller's pushed stack data; negative FP offsets are fine;
- a routine that writes `0(FP)` gets a static handler that calls the one
  stored there (§2.4.4), which is how `MOVAB handler, (FP)` keeps working;
- data in the code stream, branches into data, `CASE` without its table;
- results passed in condition codes, a `JSB` followed directly by a
  conditional branch;
- pushing, removing or rewriting a return address (`PUSHAB label` then
  `RSB`, `TSTL (SP)+`), and coroutines (`JSB @(SP)+`) unless the target
  is a declared entry point;
- `REI` to change mode, `LDPCTX`, `SVPCTX` and the other instructions that
  have no Alpha meaning;
- `PUSHR` and `POPR` in a JSB routine may double the automatic saves; the
  compiler removes what it can (§2.5.3).

Interlocked instructions become memory barriers around a load-locked and
store-conditional retry loop (§2.11.6). The compiler moves, replicates and
deletes code: a compare whose result no branch tests disappears (§1.2).

## 64-bit

MACRO-32 stays a 32-bit language; 64-bit use is opt-in and explicit
([64G] §12.1: "Make 64-bit addressing explicit in your code"). Every
source recommends a higher-level language for new code. The pieces
([64G] Table 12-1, [MCG] Table 5.1):

| Piece | What it does |
| --- | --- |
| `$SETUP_CALL64 n[, inline=]` | starts a call with n 64-bit arguments, no `#` |
| `$PUSH_ARG64 op` | puts an argument straight in its register or stack slot, in reverse order like `PUSHL`; reads aligned quadwords, quadword indexing |
| `$CALL64 target` | sets the count and calls; errors if the pushes don't match |
| `EVAX_CALLG_64 (Rn), routine` | calls with a list of quadwords in memory, a quadword count first |
| `.CALL_ENTRY quad_args=TRUE` | the routine's arguments are quadwords |
| `.ENABLE QUADWORD`, `.DISABLE QUADWORD`, `/ENABLE=QUADWORD` | address arithmetic in 64 bits |
| `EVAX_SEXTL` | sign-extends a register's low longword |
| `$IS_32BITS q, leq, gtr[, temp_reg=]` | branches on whether a quadword is a sign-extended longword |
| `$IS_DESC64 desc, target[, size=long\|quad]` | branches if a descriptor is in 64-bit form |
| `quad=YES` | the page macros (`$BYTES_TO_PAGES`, `$NEXT_PAGE`, `$PAGES_TO_BYTES`, `$PREVIOUS_PAGE`, `$START_OF_PAGE`) take 64-bit addresses |
| `$RAB64`, `$RAB64_STORE` | RMS record access blocks with buffers in 64-bit space |
| `$PUSH64 reg`, `$POP64 reg` | save and restore all 64 bits on a stack that may not be quadword aligned |

**The call macros** ([64G] §12.3.1, App. B). With more arguments than go
in registers (6 on Alpha, 8 on Itanium), `$SETUP_CALL64` builds a local
JSB routine so the stack can be octaword aligned with the arguments at the
top, unless `inline=TRUE`, which is safe only at a fixed stack depth.
Then no SP or AP reference is allowed between setup and call. The sequence
is straight-line: no branch into it, around a push or out of it.
`$PUSH_ARG64` can't be in conditional code. On Alpha, argument registers
load downward from R21, so R22-R28 are the safe temporaries; Itanium has
"no access to argument registers".

```
MOVL           8(AP), R5         ; a longword to pass
$SETUP_CALL64  3                 ; three arguments
$PUSH_ARG64    8(R0)             ; argument 3
$PUSH_ARG64    R5                ; argument 2
$PUSH_ARG64    #8                ; argument 1
$CALL64        some_routine
```

**`quad_args`** ([64G] §12.4). It doesn't force homing, and can't be
combined with `home_args`: a homed list is longwords. Code is unchanged
except in quadword instructions: with it, `MOVQ 4(AP), 8(R2)` stores all
64 bits of argument 1, without it the low longwords of arguments 1 and 2.
AP-based deferred operands load their pointer as a quadword. Routines that
share code agree on it; a JSB routine can't read a `quad_args` caller's
list. DEC suggests symbolic names for the arguments, since quadword
offsets from AP look odd.

**Address arithmetic** ([64G] §12.5). Effective addresses are computed in
longwords by default, for VAX compatibility: `4 + <1@33>` is 4. With
quadword mode they use quadword adds, at no cost. Old code may rely on
32-bit wrap: `MOVAL (R1)[R0], R2` with R1 = `7FFFFFFF` and R0 = 1 gives
`FFFFFFFF.80000003` in longword mode, `00000000.80000003` in quadword
mode. A 64-bit pointer in a register works with ordinary instructions:
`MOVL 4(R1), R0` reads the longword at R1 + 4 whatever R1 holds. The usual
ways to get one there are `EVAX_LDQ` and `MOVAx`. `MOVC3` and `MOVC5` take
64-bit addresses but lengths below 64 KB; `OTS$MOVE3` and `OTS$MOVE5` take
more.

**Built-ins** ([MCG] App. C). Any VAX operand mode works, except that a
load or store built-in's first operand is a register. Memory operands are
assumed quadword aligned, except for the byte, word and unaligned forms.
Registers must be back in sign-extended longword form before VAX
instructions read them.

| Group | Built-ins (Alpha and Itanium unless marked) |
| --- | --- |
| Sign extension | `EVAX_SEXTB`, `SEXTW`, `SEXTL` |
| Loads and stores | `EVAX_LDQ`, `STQ`, `LDAQ`, `LDBU`, `LDWU`, `STB`, `STW`, `LDQU`, `STQU` |
| Locked | `EVAX_LDLL`, `LDQL`, `STLC`, `STQC` (Itanium: `cmpxchg`) |
| Quadword arithmetic | `EVAX_ADDQ`, `SUBQ`, `MULQ`, `UMULH` |
| Logic and shifts | `EVAX_AND`, `OR`, `XOR`, `BIC`, `ORNOT`, `EQV`, `SLL`, `SRL`, `SRA`, `ZAP`, `ZAPNOT` |
| Byte manipulation | `EVAX_EXTxL`, `EXTxH`, `INSxL`, `INSxH`, x = B, W, L, Q |
| Compare | `EVAX_CMPEQ`, `CMPLT`, `CMPLE`, `CMPULT`, `CMPULE` |
| Branch | `EVAX_BEQ`, `BLT`, `BNE` |
| Conditional move | `EVAX_CMOVEQ`, `NE`, `LT`, `LE`, `GT`, `GE`, `LBC`, `LBS` |
| Barriers | `EVAX_MB`; `EVAX_TRAPB` Alpha only |
| Alpha only | `EVAX_RPCC`, `MF_FPCR`, `MT_FPCR` |
| PALcode (Itanium: system macros) | `EVAX_MTPR_x` and `EVAX_MFPR_x` for IPL, ASTEN, ASTSR, SCBB, SIRR, SISR, PCBB, the stack pointers, TB invalidates...; `EVAX_SWPCTX`, `CHMS`, `CHMU`, `BUGCHK`, `IMB`, `SWASTEN`, `READ_UNQ`, `WRITE_UNQ`; the interlocked queue calls `INSQHILR`... `REMQTIQR` |
| Itanium only | `IA64_BREAK`, `GETREG`, `SETREG`, `PROBER`, `PROBEW`, `RSM`, `SSM`, `LFETCH`... |

No source found lists x86-64 built-ins.

## The calling standards compared

| | Alpha | Itanium | x86-64 |
| --- | --- | --- | --- |
| Saved registers | R2-R15, F2-F9 | R4-R7 and others | rbx, rbp, r12-r15 |
| 32-bit values | sign-extended, unsigned too ([CS] Table 3.11) | the same: "Bit 31 is replicated in bits 32-63, even for unsigned 32-bit integers" | the same |
| Procedure value | procedure descriptor address | function descriptor {entry, GP} | the code address, "representable in 32 bits"; the linker adds trampolines |
| Bound procedure environment | R1 | R9 | %r10 |
| Current procedure | from FP: `0(FP)` holds the descriptor's address, or FP points at it ([CS] §3.5.1) | from the PC and unwind tables | from the PC and unwind tables |
| Frame description | procedure descriptor: kind, flags, register save mask and offset, frame size, prologue length, static handler | unwind info plus an OpenVMS-specific segment carrying the old flags | DWARF CFI plus compact unwind and the same OpenVMS flags |
| Below the stack pointer | 2048 bytes unpredictably modified | | a 128-byte red zone, not in kernel code |

**Condition handlers** ([CS] §9.4-9.7). On all 64-bit VMS, a handler is
associated statically with its routine, through the descriptor or the
unwind information; there is no revert. It is called with the signal and
mechanism arrays by reference. Primary, secondary and last-chance vectors
per mode are searched before and after the frames.

- The 32-bit signal array is a count, the condition, its arguments, the PC
  and the PS. The 64-bit one has the count, `SS$_SIGNAL64`, then
  quadwords: the condition, arguments, PC, PS.
- The mechanism array starts the same on every architecture: argument
  count (4 bytes), flags (4), the establisher's frame (8), depth (4),
  reserved (4), handler data (8), exception frame address (8), signal
  array address (8), the return values R0 and R1 (8 each, at 48 and 56).
  Then the architecture's scratch registers, on Alpha R16-R28 at 64-160,
  "R2 through R15 are implicitly saved in the call stack", and the 64-bit
  signal array's address. A handler may change only the return values,
  through `SYS$SET_RETURN_VALUE`.
- Depth 0 is the frame that signaled, 1 its caller; the vectors are −1 to
  −3. `$UNWIND(0, 0)` returns to the establisher's caller; each removed
  frame's handler is called with `SS$_UNWIND` first.

## 64-bit addresses

**The address space** ([64G] Ch. 2). P0 (`0`-`3FFFFFFF`) and P1
(`40000000`-`7FFFFFFF`) as on the VAX; P2 from `00000000.80000000` up to
page table space; S2 below S0/S1, which sit at `FFFFFFFF.80000000`-
`FFFFFFFF.FFFFFFFF` so a sign-extended longword still reaches them, and
bit 31 still tells process from system addresses. Page tables live in a
page table space at the same address in every process. "There are no
special 64-bit processes or 32-bit processes": 32-bit and 64-bit code mix
freely, joined by sign extension.

**Services** ([64G] Ch. 3, Ch. 8). A 32-bit service takes 32-bit
addresses, sign-extended. A "64-bit friendly" one needs no change: `$QIO`,
`$SYNCH`, `$ENQ`, `$FAO`, since arguments were always 64 bits wide and
descriptors and RMS blocks identify their own form. Services whose
interface can't hold a 64-bit address got `_64` twins: `$CRETVA_64`,
`$EXPREG_64`, `$CRMPSC_FILE_64`, `$CMKRNL_64`, and new region services
such as `$CREATE_REGION_64`. The suffix marks a 64-bit address passed by
reference. Every service not enhanced for 64 bits, user-written ones
included, gets sign-extension checking: an argument that isn't a
sign-extended longword returns `SS$_ARG_GTR_32_BITS`. MACRO-32 has no
macros for `_64` services; it calls them with `$CALL64` or
`EVAX_CALLG_64` (§3.4). Rules for new interfaces: pass addresses, sizes
and lengths by value as quadwords; return allocated memory by value; never
return a 64-bit address to a caller that didn't ask for one.

**Self-identifying structures** ([64G] §8.1, [CS] §8.1). A 64-bit
descriptor has a word MBO = 1 where the 32-bit length is, and a longword
MBMO = −1 where the 32-bit pointer is, so old code sees a 1-byte string at
`FFFFFFFF` and fails cleanly:

| Offset | 32-bit descriptor | 64-bit descriptor |
| --- | --- | --- |
| 0 | word length | word MBO = 1 |
| 2 | byte type, byte class | byte type, byte class |
| 4 | longword pointer | longword MBMO = −1 |
| 8 | | quadword length |
| 16 | | quadword pointer |

A routine tests both fields, since a 32-bit descriptor of length 1 could
pass the MBMO test alone. The 64-bit item lists `item_list_64a` and
`item_list_64b` use the same trick: word MBO, word item code, longword
MBMO, quadword length, quadword buffer address, and in the `b` form a
quadword return length address. RMS's `RAB64` extends the RAB with
quadword buffer fields, used when the longword field holds −1; only the
user, record, header and key buffers may be in P2 or S2 (Ch. 5).

## What vmacro takes

DESIGN-0004 is the result. Against the sources:

- **Taken as is.** Sign-extended longwords in registers; declared routines
  with automatic 64-bit preservation of what they modify, R0 and R1
  excepted, and `.JSB32_ENTRY` as the opt-out; `input`, `output`,
  `scratch`, `preserve`, `max_args`, `home_args`, `quad_args`; `n(AP)`
  mapped to where the arguments arrive, with homing on DEC's triggers;
  the flagged idioms; JSB as a native call, with no return address on the
  VAX stack; the 64-bit pieces, under DEC's names, mapped onto ARM64 as VSI
  mapped them onto Itanium; sign extension of every 32-bit argument and
  result; the argument count; the 64-bit mechanism array, descriptors and
  item lists; sign-extension checks in the service dispatcher.
- **Changed for ARM64.** R2-R11 go to x19-x28, since AAPCS64's saved
  registers are those, as Itanium remapped to fit Intel's. Eight arguments
  in registers, so the call macros' threshold is 8, as on Itanium. VAX SP
  stays a register of its own, x18, rather than `sp`, because EL0 checks
  `sp`'s alignment on every access. Arguments arriving in x0 and x1, which
  are also R0 and R1, are copied at entry when the routine reads them. The
  frame keeps AAPCS64's record at `0(FP)`, so the descriptor's address
  goes at `24(FP)` and the handler at `16(FP)`. The 64-bit address space
  can't put S0 at `FFFFFFFF.80000000`: seL4 keeps the upper half, so
  vaxpunk's S0 is below 2 GB (ADR-0005) and bit 31 doesn't tell process
  from system addresses.
- **Left out.** Alpha's linkage sections and procedure descriptors as
  procedure values (ARM64 is PC-relative); `.LINKAGE_PSECT`,
  `.DEFINE_PAL`, `EVAX_TRAPB`, `RPCC`, the FPCR built-ins and the
  Itanium-only built-ins; `.CALL_LINKAGE` and its family, which Itanium
  needed because its mapping put VAX registers on scratch registers, and
  vaxpunk's doesn't (accepted and ignored, so ported sources compile);
  Alpha's byte-manipulation built-ins until something needs them.
