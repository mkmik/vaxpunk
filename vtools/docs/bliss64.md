# BLISS-64 and BLISS-32: the dialects vbliss compiles

vbliss ([PRD-0004](../../docs/prd/0004-bliss64-compiler.md)) compiles
the two dialects of DEC's BLISS compilers for OpenVMS Alpha: BLISS-64, as
BLISS-64EN compiled it, with `/A64`, the default; and BLISS-32, as
BLISS-32EN compiled it, with `/A32`, so that existing 32-bit BLISS code
recompiles as it did on Alpha. The language itself is the BLISS Language
Reference Manual's: common BLISS, plus its BLISS-32 parts. The manual is
from 1987 and knows nothing of Alpha, so this file lists what the Alpha
dialects change, add and drop, where each item was learned, and how
vaxpunk maps it onto ARM64 and the
[calling standard](../../docs/design/0004-calling-standard.md). The
items are BLISS-64's; *BLISS-32 on vaxpunk* says what `/A32` does
differently.

Anything this file doesn't mention is as the manual says for BLISS-32.

## Sources

Each item below carries the key of its source.

| Key | Source |
| --- | --- |
| LRM | *BLISS Language Reference Manual*, AA-H275E-TK, May 1987: `blslref.pdf` on the OpenVMS Freeware CDs (`bliss/`, also in the kit) |
| UM | *BLISS-32 User Manual*, AA-H322E-TE, May 1987: `b32uman.pdf`, same places |
| RN | *Release Notes for Alpha BLISS V1.11-007*, chapter 2, "Differences between BLISS-32 and Alpha BLISS": `[SYSHLP]BLSA64111-007.RELEASE_NOTES` in the kit `hp-axpvms-blissa64-v0111-7-1.pcsi` (Freeware V8.0) |
| KIT | The compiler's own tables of reserved words, built-ins, lexical functions and switches, read as strings from the same kit |
| ARCH | `rebuilding_starlet.txt`, next to the kits on the Freeware CD: how STARLET.L64 and LIB.L64 are built, and the `ARCH_DEFS` macros |
| VSI | VSI's release notes for BLISS V1.15-148 on OpenVMS x86-64: how VSI retargeted the same dialect to another architecture |
| probe | Not stated by any document; to be confirmed against BLISSA64 V1.11-7 in AXPbox once the oracle harness (PRD-0004 step 3) exists |
| vaxpunk | A choice of ours, where BLISS-64 is tied to Alpha |

The kit stores the release notes as text in variable-length records (a
16-bit length, the bytes, padded to even) with a 4-byte PCSI chunk header
(`04 82 3E 00`) every 15,876 bytes; dropping the headers and reading the
records gives the text back.

The Alpha BLISS kit carries no BLISS-64 manual. The release notes are the
only DEC document of the dialect: DEC's manuals stayed the two above, and
the notes say how the Alpha compilers differ from them. The online help is
listed as missing in the notes' known bugs.

## The compiler

- **Two Alpha compilers, one front end.** BLISS-32EN does operations 32
  bits wide, BLISS-64EN 64 bits wide; both produce Alpha objects and share
  the reserved words, switches and built-ins below. `BLISS/A64` runs
  BLISS-64EN, and `BLISS/A32`, or plain `BLISS`, BLISS-32EN. (RN 2.1)
- **File types.** BLISS-64EN looks for sources as `.B64E`, `.B64`, `.BLI`,
  require files as `.R64E`, `.R64`, `.REQ`, libraries as `.L64E`, `.L64`,
  `.LIB`; BLISS-32EN for `.B32E`, `.B32`, `.BLI`, `.R32E`, `.R32`,
  `.REQ` and `.L32E`, `.L32`, `.LIB`. Libraries don't move between
  dialects. vbliss takes the same lists without the `E` types.
  (RN 2.2.1; vaxpunk)
- **Dialect tests.** `%BLISS(BLISS64E)` and `%BLISS64E(...)` are true in
  BLISS-64EN; `%BLISS(BLISS32)` is true in every 32-bit compiler and
  `%BLISS(BLISS32V)` only on the VAX. `%HOST` and `%TARGET` take an
  architecture (`VAX`, `MIPS`, `ALPHA`, also `IA64`, `INTEL` in the
  table) and an operating system (`VMS`, `UNIX`, `OSF`, `WNT`...), and an
  unknown keyword is 0, not an error. VSI added `BLISS64X` and `X86_64`
  for x86-64. `ARCH_DEFS.REQ`'s `ALPHA` macro is
  `%BLISS(BLISS32E) OR %BLISS(BLISS64E)` and `ADDRESSBITS` is `%BPADDR`.
  (RN 2.12; KIT; VSI; ARCH)

  There is no plain `BLISS64` keyword: the table has `BLISS16`,
  `BLISS32`, `BLISS36` and the `BLISS32x`/`BLISS64x` variants only, so
  64-bit code tests `%BPVAL` or a variant. (KIT)

  vaxpunk: `%BLISS(BLISS64E)` and `%BLISS(BLISS32E)` are false, as VSI
  made them on x86-64, since code under them assumes Alpha's registers and
  frames; `%BLISS(BLISS32)` is true under `/A32`. vbliss gets a variant
  of its own for each dialect and `%TARGET(ARM64)`, `%TARGET(VMS)` true;
  the variants' names (`BLISS64A`, `BLISS32A`?) are open until DEC-style
  code needs them.
- **Syntax levels.** `/SYNTAX_LEVEL=1` has the manual's reserved words;
  level 2, the default, adds `QUAD`, `SHARED`, `ALIAS`, `EXTERNAL_NAME`
  and `NOCHECK_ALIGNMENT`; level 3 adds `VARIABLE`. The table also
  reserves `GRANULARITY`. vbliss takes `/SYNTAX_LEVEL` and defaults to 2,
  as DEC did, since old code may use these words as names.
  (RN 2.19.11; KIT; vaxpunk)
- **Limits.** 256 actual parameters (64 on the VAX), names up to 64
  characters, a `CASE` range up to 1024. vbliss allows 255 actuals, all
  that x9's count holds (DESIGN-0004 *Arguments*). (RN 2.23; vaxpunk)

## Values and sizes

The core of the dialect: a BLISS value, the fullword, is a quadword.

| Literal | BLISS-32, `/A32` | BLISS-64, `/A64` |
| --- | --- | --- |
| `%BPVAL` | 32 | 64 |
| `%BPUNIT` | 8 | 8 |
| `%BPADDR` | 32 | 64 |
| `%UPVAL` | 4 | 8 |

(LRM 1.5 for BLISS-32; RN 2.1 and ARCH for 64-bit values and addresses;
`%UPVAL` = `%BPVAL/%BPUNIT`, probe)

- **QUAD** is an allocation unit of 8 bytes, allowed wherever `BYTE`,
  `WORD` and `LONG` are. The 32-bit compilers don't have it. (RN 2.4)
- **The default allocation unit** is QUAD: a scalar, a `VECTOR`, `BLOCK`
  or `BLOCKVECTOR` element, a `PLIT` or `INITIAL` item with no unit, and a
  fetch or store with no size is 64 bits. (RN 2.4, 2.19.1.2)
- **`LONG_DEFAULT`** (switch, or `/ASSUME=LONG_DEFAULT`) makes all of the
  above LONG, the `PLIT` count a longword, and `ARGPTR`'s vector one of
  longwords. **`REF_LONG`** makes every `REF` structure variable a signed
  longword. **`SIGNED_LONG`** is both, with scalars and `VECTOR` elements
  LONG SIGNED. They exist to move BLISS-32 code to BLISS-64 with the
  fewest changes, and vbliss has all three, as module switches and as
  `/ASSUME`. (RN 2.19.1.2-4; vaxpunk)
- **Shifts.** `^` is defined for counts -63 to 63. `SLL`, `SRL`, `SRA`
  shift in one direction by 0 to `%BPVAL`-1. (RN 2.17, 2.9.5)
- **Field size** in `<P,S,E>` goes to `%BPVAL`, 64. (LRM 11.10.3, with
  `%BPVAL` = 64)
- **Literals.** A number that doesn't fit a fullword is an informational
  "numeric literal overflow"; in BLISS-64 that is past 64 bits. (RN 2.24)

### Signs

Field values are zero-extended unless their extension says otherwise
(`E` = 1, `SIGNED`), as in BLISS-32; the default for a `VECTOR` is
unsigned (LRM 11.10.1). In BLISS-32 a longword is the fullword and has no
extension to choose. In BLISS-64 it has one, and a LONG declared without
`SIGNED` is zero-extended when fetched. RN gives `/CHECK=LONGWORD` to find
LONG declarations that say neither `SIGNED` nor `UNSIGNED`, for that
reason. (RN 2.19.5)

On vaxpunk this meets the calling standard's rule that a 32-bit value is
sign-extended across a call (DESIGN-0004 *Values*): a status from a
MACRO-32 routine arrives sign-extended, while the same status stored in an
unsigned LONG and fetched comes back zero-extended, and the two compare
unequal when bit 31 is set. vbliss sign-extends nothing on its own; the
lint and `/CHECK=LONGWORD` are where this gets caught. (vaxpunk)

## Structures

The predeclared structures are the manual's with `%UPVAL` = 8 (LRM
11.10):

```
STRUCTURE
    VECTOR[I; N, UNIT=8, EXT=0] = [N*UNIT] (VECTOR+I*UNIT)<0,8*UNIT,EXT>;
    BLOCK[O, P, S, E; BS, UNIT=8] = [BS*UNIT] (BLOCK+O*UNIT)<P,S,E>;
```

and `BLOCKVECTOR` likewise. So `VECTOR[10]` is 80 bytes and `BLOCK[4]` 32,
and offsets in a `BLOCK` without a unit count quadwords. VMS code declares
its blocks `BLOCK[n, BYTE]`, as on the VAX, so this rarely shows. `VECTOR`
takes `QUAD` as a unit. (probe for the exact text of each; LRM for the
form)

- **BITVECTOR** is BLISS-32's, `BITVECTOR<I,1>` over `(N+7)/8` bytes.
  (LRM 11.10.2; probe)
- **BLOCK_BYTE** is new, predeclared: `BLOCK[,BYTE]` under another name,
  `[BS] (BLOCK_BYTE+O)<P,S,E>`. (RN 2.32)
- **`REF` with attributes.** `ALIGN` and `VOLATILE` written between `REF`
  and the structure name apply to the structure pointed to, not to the
  pointer: `REF VOLATILE BLOCK`. Before `REF` or after the structure they
  keep their BLISS-32 meaning. (RN 2.25)
- **Field attributes.** A field can be `VOLATILE` (every access through
  it is) or `NOCHECK_ALIGNMENT`: `FOO1 = [0,0,32,0] : VOLATILE`. (RN 2.28)
- **A structure's first formal in a conditional** compiles wrongly in
  BLISSA64; the fix is to name the structure once outside the condition.
  vbliss must get it right, and the oracle can't be trusted there.
  (RN chapter 4, item 2)

## Data

- **PLIT.** The count before a `PLIT` is a fullword, so a quadword, and
  counts fullwords: quadwords of data. Items with no unit are quadwords.
  (LRM 4.4 and RN 2.19.1.2, which says only `LONG_DEFAULT` makes the count
  a longword; probe for the layout)
- **`%ASCID`** makes a string descriptor in the `PLIT` psect and is its
  address. The manual gives BLISS-32's: a word of length, a byte of type
  (14, `DSC$K_DTYPE_T`), a byte of class (1, `DSC$K_CLASS_S`), a longword
  pointer. Whether BLISS-64 makes the same 8-byte descriptor or a 64-bit
  one, nothing says. vaxpunk's services take the 32-bit one. (LRM 4.3;
  probe)
- **Link-time constants.** On OpenVMS Alpha a `PLIT` item, `PRESET` or
  `INITIAL` of static data may only be `e1-e2`, each a constant, a symbol,
  or a symbol plus or minus a constant: fewer forms than on the VAX.
  vbliss accepts those and what vlink's relocations express. (RN 2.15;
  vaxpunk)
- **Alignment.** `ALIGN` is also allowed on `EXTERNAL`, `BIND` and
  `GLOBAL BIND`, where it tells the compiler what it may assume about data
  it doesn't allocate. `BLOCK_ALIGNMENT(FULLWORD)`, the default, lets the
  compiler assume a `REF BLOCK` and a `BIND` to a `BLOCK` are 8-byte
  aligned; `NATURAL` assumes each field is aligned as its offset allows.
  A wrong assumption costs an alignment fault, never a wrong result.
  vbliss accepts them and assumes nothing: ARM64 loads and stores work
  unaligned on normal memory, and only atomics need the alignment.
  (RN 2.5, 2.31; vaxpunk)
- **Granularity.** Alpha writes bytes and words only through longwords, so
  BLISS-64 has `GRANULARITY(n)`, the `DEFAULT_GRANULARITY` switch and
  `/GRANULARITY`, to ask for byte-safe writes. ARM64 writes bytes and
  words on their own: vbliss accepts and ignores them, as VSI's x86-64
  compiler does. (RN 2.16.1; VSI; vaxpunk)
- **`ADDRESSING_MODE`** is accepted and ignored. (RN 2.21)

### Psects

- Code and data never share a psect. The code psect defaults to `SHARE`,
  `NOREAD`. Psects default to octaword alignment. The absolute psect is
  `$ABS$` and the literal one `$LITERAL$`. (RN 2.20)
- Besides BLISS-32's `$CODE$`, `$PLIT$`, `$OWN$` and `$GLOBAL$`, every
  module is as if inside two more declarations:
  `LINK = $LINK$ (READ, NOWRITE, NOEXECUTE, NOSHARE, NOPIC, CONCATENATE,
  LOCAL, ALIGN(3))` for Alpha's linkage section, and `INITIAL = $INITIAL$
  (NOWRITE, NOEXECUTE, CONCATENATE, LOCAL)` for the constant data of
  `LOCAL` initial values. (RN 2.22, 2.19.10)

  vaxpunk: ARM64 code reaches its data PC-relative and has no linkage
  section (DESIGN-0004 *Procedure values*). vbliss accepts `LINK =` and
  puts nothing in it; `$INITIAL$` is kept. Every routine with a frame
  also has a frame descriptor in `$CODE$_FDSC`, as `call.mlb`'s
  `$ROUTINE` puts it.
- `GP_RELATIVE` is a psect attribute on Itanium and x86-64 for short
  data. Not in vbliss. (KIT; VSI)

### Attributes

- **ALIAS** says a variable can change through other names: pointers, item
  lists, calls. The Alpha compilers mark ALIAS on their own any variable
  whose address is used other than to fetch or store it, and
  `/CHECK=ADDRESS_TAKEN` reports each one ("assuming ALIAS"). vbliss
  keeps a variable whose address is taken, or that is ALIAS, in memory and
  never in a register; the address-taken report is part of the dot lint.
  (RN 2.6, 2.19.3; vaxpunk)
- **VOLATILE** is stronger than on the VAX: exactly one access of the
  declared size for each fetch and store, in source order, aligned. That
  is what device registers need. vbliss does this for every access
  anyway; VOLATILE also forbids it to keep the value in a register.
  (RN 2.7)
- **EXTERNAL_NAME('name')** gives the linker name, case kept, on `GLOBAL`,
  `EXTERNAL`, routines, literals and `GLOBAL BIND`; the argument works
  like `%STRING`. Names without it are upper-cased, which `/NAMES` can
  change. (RN 2.14, 2.19.9)
- **Duplicates** of `VOLATILE`, `ALIAS`, `NOVALUE` and `WEAK` are allowed
  without a message. (RN 2.27)
- **Routine attributes for the value.** A routine can say what it returns:
  an allocation unit, `SIGNED`/`UNSIGNED`, a `FIELD`, a `[REF]`
  structure. `VARIABLE` (level 3) lets calls pass a different number of
  actuals than the declaration has formals. (RN 2.29)
- **Prototypes.** `EXTERNAL ROUTINE` and `FORWARD ROUTINE` can list
  their formals, and `%BLISS(PROTOTYPES)` says the compiler allows it;
  `/CHECK=PARAMETERS` checks `REF` structure formals against the size of
  undotted actuals. (RN 2.19.8; KIT) These are what the dot lint's rule 6
  needs across modules.

## Linkages

Linkage types: `CALL`, `JSB`, `INTERRUPT`, and new, `EXCEPTION`.
`INTERRUPT` and `EXCEPTION` routines can't be called from BLISS. The kit's
table also has `PORTAL`, `SKIP` and the Windows NT ones (`CDECL`,
`FASTCALL`); they're not in the release notes and not in vbliss. (RN 2.8;
KIT)

- **CALL** is the platform's calling standard; BLISS-64 and BLISS-32 code
  call each other, values truncated or sign-extended across. It passes an
  argument count unless the linkage says `NOCOUNT` (or the module
  `NOCOUNT`, or `/NOCOUNT`); a `NOCOUNT` routine can't use
  `ACTUALCOUNT`, `ACTUALPARAMETER`, `NULLPARAMETER` or `ARGPTR`.
  (RN 2.8.1, 2.8.5)

  vaxpunk: CALL is DESIGN-0004: arguments in x0-x7 and on the stack, the
  count in x9's bits 7:0. `NOCOUNT` leaves x9 unset, as a call into C would.
- **JSB** routines are frameless: they don't change FP. (RN 2.8.2)

  vaxpunk: `bl` and `ret`, as DESIGN-0004 *JSB* says. A JSB routine that
  calls saves x30 with the registers it preserves.
- **Registers.** Linkage register numbers name Alpha's integer registers,
  0-30, not the VAX's 0-11; floating registers can't be named. Many are
  reserved, and asking for one is a "register conflict". The names `AP`,
  `FP`, `SP` and `PC` are errors: `AP` doesn't exist, and the others mean
  Alpha registers 29 and 30, which the notes advise touching only in
  assembler. `BUILTIN R29` and the like name Alpha registers. VSI's x86-64
  compiler keeps Alpha's numbering and maps it, and its
  `ALPHA_REGISTER_MAPPING` switch changes nothing there. (RN 2.3.2-2.3.5,
  2.9.1; VSI 1.5; KIT)

  vaxpunk: register numbers are VAX R0-R11, mapped as vmacro maps them
  (R0, R1 to x0, x1; R2-R11 to x19-x28; `vtools/docs/macro32.md`), so a
  BLISS-64 linkage states a MACRO-32 routine's register contract as
  MACRO-32 states it. Numbers 12-30 are an error. This departs from
  BLISS-64, whose numbers are Alpha's: Alpha's R0-R11 are VAX R0-R11 under
  AMACRO, so linkages to MACRO-32 routines read the same, but one naming
  Alpha's argument registers (16-21) won't port.
- **Global registers.** A `GLOBAL` register in a linkage that the routine
  doesn't declare `EXTERNAL REGISTER` is preserved; one it declares
  `GLOBAL REGISTER` again is preserved and starts a new lifetime,
  BLISS-32's rules. `/CHECK=REDECLARE` points the second case out.
  (RN 2.8.3, 2.30)
- `PRESERVE`, `NOPRESERVE`, `NOTUSED` and output parameters are BLISS-32's
  (LRM 13.3). Under BLISS-32's CALL, R0 and R1 aren't preserved and R2-R11
  are (LRM 13.3.3); under DESIGN-0004 x0, x1 aren't and x19-x28 are, the
  same registers.

### Linkage functions

- `ACTUALCOUNT`, `ACTUALPARAMETER(i)` and `NULLPARAMETER(i)` are as in
  common BLISS. (LRM 13.6; RN 2.3.2)
- `ARGPTR()` returns the address of a VAX-style argument list the
  compiler builds: a count, then the actuals, a fullword each, quadwords
  in BLISS-64. They are copies; storing into the list changes nothing.
  (RN 2.3.2)
- `RETURNADDRESS` is in the table. (KIT)
- There is no `CALLG` built-in. DEC's replacement for `CALLG(.AP, rtn)` was
  `BLI$CALLG(ARGPTR(), rtn)`, a routine of theirs, for BLISS-32EN only on
  Alpha; VSI dropped it on x86-64 for `LIB$CALLG`. vbliss calls
  `LIB$CALLG`, which DESIGN-0004 *Argument lists* already plans.
  (RN 2.3.2; VSI 1.5.3; KIT)

## Condition handling

- `ENABLE`, `SIGNAL`, `SIGNAL_STOP` and `SETUNWIND` are common BLISS's.
  (LRM chapter 17)
- **`ESTABLISH(rtn)` and `REVERT()`** are new: they set and clear the
  routine's handler at run time, as `.FP = handler` did on the VAX, which
  is no longer allowed. Not in the same routine as `ENABLE`; such a
  handler gets no enable vector, and goes away when the routine returns.
  (RN 2.3.3)
- **`SIGNALREF` and `SIGNALREF_STOP`** take the address of a 64-bit
  condition value and are otherwise `SIGNAL` and `SIGNAL_STOP`. (RN
  2.33.3)
- `RETURN_UNWIND` exists only on Tru64. (RN 2.33.2)
- An `ENABLE`d handler is called through a jacket in the run-time library,
  `OTS$BLISS_STATIC_HANDLER` (and `OTS$BLISS_DYNAMIC_HANDLER` for
  `ESTABLISH`), which adds the enable vector to the arguments the system
  passes. (KIT: the names are in the compiler's table; how they are used,
  probe)

  vaxpunk: `ENABLE` writes the handler at `16(FP)` (DESIGN-0004
  *Condition handling*). The enable vector needs the same kind of jacket,
  which vbliss's run-time support supplies; `ESTABLISH` is a store to
  `16(FP)` and `REVERT` clears it.

## Built-ins

The VAX's machine built-ins are gone: no `MOVC3`, `INSQUE`, `MTPR`,
`PROBER`, `CALLG`, `EMUL`, `FFS`. Their replacements are the `CH$`
functions, the linkage functions, common built-ins like `ROT`, the Alpha
built-ins and the PAL built-ins below. (RN 2.3.1; KIT, where none of the
VAX names appears)

**PAL calls.** Each Alpha PAL call is a built-in named `PAL_` and the
call: `PAL_MTPR_IPL`, `PAL_MFPR_PCBB`, `PAL_PROBER`, `PAL_CHMK`,
`PAL_INSQHIL`, `PAL_REMQTIQ`, `PAL_SWPCTX`, `PAL_HALT`... (the full list
is RN 2.9.2 and the kit's table). `CALL_PAL(code, args...)` makes any
call, the code a compile-time constant. Inputs and outputs are the SRM's.
(RN 2.9.2)

vaxpunk: the PAL keeps Alpha's numbers (DESIGN-0001), so each `PAL_x`
built-in whose call vaxpunk implements is that call, and `CALL_PAL`
reaches vaxpunk's own (0x40 and up). One that the PAL doesn't implement
is a compile error. The interlocked queue calls (`PAL_INSQHIL` and the
rest) need the PAL calls first.

**Atomics and barriers** (RN 2.9.3-2.9.6, 2.13):

- `ADD_ATOMIC_LONG`/`_QUAD`, `AND_`, `OR_`: `(ptr, expr [, retries] [;
  old])`, 1 if done within `retries` tries. `ptr` must be naturally
  aligned.
- `CMP_STORE_LONG`/`_QUAD(addr, comparand, value, dest)`, and
  `CMP_SWAP_LONG`/`_QUAD` (KIT; VSI 1.5).
- `TESTBITSS`, `TESTBITSC`, `TESTBITCS`, `TESTBITCC` (atomic against
  ASTs), `TESTBITSSI`, `TESTBITCCI` (interlocked), `ADAWI` (returns VAX
  condition codes, faked).
- `BARRIER`: reads before reads, writes before writes, reads before
  writes.

vaxpunk: `ldxr`/`stxr` loops and `dmb`.

**Other machine built-ins**: `ROT`, `SLL`, `SRL`, `SRA`, `UMULH`,
`CMPBGE`, `ZAP`, `ZAPNOT`, `TRAPB`, `DRAINT`, `RPCC`, `WRITE_MBX`.
vaxpunk: `ROT`, the shifts and `UMULH` map to single instructions;
`CMPBGE`, `ZAP`, `ZAPNOT` are byte-mask operations, done in a few;
`TRAPB` and `DRAINT` are no-ops; `RPCC` reads `CNTVCT_EL0`; `WRITE_MBX`
has no user. (RN 2.9.6; KIT)

**Floating point.** Built-ins for F, D, G, S and T arithmetic and
conversion (`ADDT`, `CVTTQ`...), `%S` and `%T` literals, `%FFLOAT` to
`%TFLOAT` to pass and return floating values by value, and
`ENVIRONMENT(NOFP)`. Not in vbliss until vasm has floating point
instructions. (RN 2.10, 2.11; vaxpunk)

## Lexical functions

New: `%HOST`, `%TARGET`, `%MODULE`, `%ROUTINE`, `%IDENT`, `%BLISS32E`,
`%BLISS64E`, `%BLISS32V`. (RN 2.12) The kit's table has these too,
which the manual (LRM chapters 15 and 16) mostly has: `%QUOTE`, `%UNQUOTE`,
`%EXPAND`, `%REMAINING`, `%COUNT`, `%LENGTH`, `%NUMBER`, `%STRING`,
`%CHAR`, `%CHARCOUNT`, `%EXPLODE`, `%REMOVE`, `%NAME`, `%QUOTENAME`,
`%ISSTRING`, `%NULL`, `%IDENTICAL`, `%DECLARED`, `%SWITCHES`, `%VARIANT`,
`%CTCE`, `%LTCE`, `%NBITS`, `%NBITSU`, `%SIZE`, `%ALLOCATION`,
`%FIELDEXPAND`, `%ASSIGN`, `%EXACTSTRING`, `%ERROR`, `%WARN`, `%INFORM`,
`%PRINT`, `%MESSAGE`, `%ERRORMACRO`, `%EXITITERATION`, `%EXITMACRO`,
`%TITLE`, `%SBTTL`, `%REQUIRE`, `%CURNAME`, `%IF`/`%THEN`/`%ELSE`/`%FI`,
and the string forms `%ASCII`, `%ASCIZ`, `%ASCIC`, `%ASCID`, `%RAD50_11`,
`%RAD50_10`, `%SIXBIT`, `%DECIMAL`. `%RAD50_10` and `%SIXBIT` belong to
BLISS-36 and `%RAD50_11` to BLISS-16: vbliss rejects them. (KIT; vaxpunk)

## Switches and qualifiers

Module switches the kit knows, besides the manual's: `OVERFLOW`,
`ALPHA_REGISTER_MAPPING`, `BLOCK_ALIGNMENT`, `DEFAULT_GRANULARITY`,
`COUNT`, `LONG_DEFAULT`, `REF_LONG`, `SIGNED_LONG`, and a `CHECK_x` switch for
each `/CHECK` option: `ADDRESS_TAKEN`, `ALIGNMENT`, `FIELD`, `LONGWORD`,
`OPTIMIZE`, `PARAMETERS`, `REDECLARE`, `SHARE`, `SHORT_ADDRESS`.
(RN 2.26; KIT)

The checks worth having in vbliss, because they find the bugs 64-bit
fullwords bring:

- `LONGWORD`: LONG without `SIGNED` or `UNSIGNED`; a 64-bit value stored
  in a 32-bit field or passed where a routine takes 32 bits. (RN 2.19.5)
- `SHORT_ADDRESS`: a fetch or store through an address held in less than
  64 bits, or an address stored in less than 64 bits. On vaxpunk, below
  2 GB, this is harmless until P2 space; off by default as in BLISSA64.
  (RN 2.19.7)
- `ADDRESS_TAKEN`: see ALIAS above; the dot lint's base.

Qualifiers that don't carry over: `/TIE`, `/GRANULARITY`, `/OPTIMIZE`
levels and `/ENVIRONMENT`. BLISS-32's `/CROSS_REFERENCE`, `/DESIGN`,
`/QUICK` aren't in Alpha BLISS either. `/MACHINE_CODE_LIST` is yes or no.
`/INCLUDE=(dir,...)` adds directories for `REQUIRE` and `LIBRARY`, and
then the file type must be written out. An output qualifier after an
input file puts the output next to it. (RN 2.19.13-15, 2.2.2)

## BLISS-32 on vaxpunk

`/A32` is BLISS-32EN: the manual's BLISS-32, with the Alpha changes in
this file except those that come from 64-bit values. On Alpha it was how
existing BLISS code moved over: unchanged where it kept to common BLISS,
edited where it used the VAX. (RN 2.1, 2.3)

- **Values are 32 bits.** vbliss computes them in 32-bit registers, the
  IR's `w` class, and keeps them sign-extended in 64-bit ones, as vmacro
  keeps a longword, so they cross calls as DESIGN-0004 *Values* wants.
  (vaxpunk)
- **Addresses are 32 bits** and stored in longwords, so an image with
  BLISS-32 code must lie below 2 GB, as one with MACRO-32 code must.
  (vaxpunk; `vtools/docs/macro32.md`)
- **Sizes are BLISS-32's.** No `QUAD` allocation unit; LONG is the
  default unit; the predeclared structures have `UNIT=4`; a `PLIT`'s
  count is a longword; `ARGPTR`'s list is longwords; `%ASCID` makes
  BLISS-32's 8-byte descriptor. (LRM 4.3, 4.4, 11.10; RN 2.3.2, 2.4)
- **Between dialects.** A BLISS-64 routine called from BLISS-32 code gets
  each value sign-extended; a BLISS-32 routine sees the low 32 bits of
  what a BLISS-64 caller passes. `SIGNALREF` signals a 64-bit condition
  value from BLISS-32. The `_QUAD` atomics work, the operand
  sign-extended and the old value truncated. (RN 2.8.1, 2.9.3, 2.33.3)
- **BLISS-64 only:** `LONG_DEFAULT`, `REF_LONG`, `SIGNED_LONG`,
  `/CHECK=LONGWORD` and `/CHECK=SHORT_ADDRESS`. (RN 2.19)
- **Linkages** name VAX R0-R11 as under `/A64` (*Linkages*), which here is
  what the code was written for: a VAX BLISS-32 `JSB` linkage to a
  MACRO-32 routine compiles unchanged.

What porting VAX BLISS-32 code still takes, as it did on Alpha (RN 2.3):

- The VAX built-ins become the `CH$` functions (`MOVC3` is `CH$MOVE`),
  the linkage functions, `PAL_` built-ins (`MTPR(x, PR$_IPL)` is
  `PAL_MTPR_IPL(x)`, `INSQHI` is `PAL_INSQHIL`), or calls:
  `CALLG(.AP, rtn)` is `LIB$CALLG(ARGPTR(), rtn)`.
- `AP`, `FP`, `SP` and `PC` are gone: `ACTUALPARAMETER` for `AP`,
  `ESTABLISH(rtn)` for `.FP = rtn`; code that reads VAX frames is
  rewritten.
- What Alpha needed for byte and word writes, ARM64 doesn't.

Open: vbliss could accept the commonest VAX built-ins under `/A32`
(`CALLG`, `MOVC3`, `MOVC5`, `INSQUE`, `REMQUE`, `FFS`, `FFC`, `EMUL`,
`EDIV`) to make porting cheaper still. DEC didn't; the first BLISS-32
module worth porting shows which ones would pay.

## STARLET and LIB

DEC built `STARLET.L64` from `STARLET.R64`, and `LIB.L64` from `LIB.R64`
with `STARLET.REQ` and `STARLET.R64`: the 64-bit libraries have require
files of their own, because the macros that use `QUAD` (the 64-bit RMS
blocks, for one) can't be read by the 32-bit compilers. vdefs writes
`STARLET.R64` and `LIB.R64` from vaxpunk's macro libraries
(PRD-0004 *Definitions*), and `STARLET.REQ` and `LIB.REQ` for `/A32`,
from which DEC built `STARLET.L32` and `LIB.L32`. (ARCH; RN 5.1.1)

## To probe

What the oracle harness must settle before vbliss relies on it, each with
a test program whose listing or output answers it:

1. `%UPVAL`, `%BPADDR`, and `%ALLOCATION` and `%SIZE` of a scalar,
   `VECTOR[3]`, `BLOCK[2]`, `BITVECTOR[9]`, `BLOCKVECTOR[2,3]`.
2. A `PLIT`'s layout: the count's size and value, item sizes, padding
   between `BYTE` items and the next fullword; `UPLIT` without the count.
3. `%ASCID`: the descriptor's size and fields.
4. Fetching -1 from `LONG`, `LONG SIGNED`, `WORD` and `BYTE SIGNED`
   scalars and fields, printed as signed and unsigned 64-bit values.
5. `ARGPTR()`'s list for 3 actuals, and whether the 4th slot is read
   past the count.
6. Which linkage register numbers BLISSA64 rejects as a register conflict.
7. `INCR`/`DECR`/`INCRU` loops across 2^63 and 2^32; `^` by 63, 64 and
   -64; `LSS`/`LSSU` on values with bit 63 or bit 31 set.
8. The predeclared structures as the listing shows them under
   `/SHOW=EXPANSIONS`, if it does.
9. Whether BLISSA64 still miscompiles a structure formal first used in a
   conditional (RN chapter 4).
10. Under `/A32`: items 1 to 4, and what a BLISS-32 routine receives
    from a BLISS-64 caller passing 2^32+5 and -1.
