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
| CLD | `[SYSUPD]BLISS_AN.CLD` in the same kit: the DCL definition of the `BLISS` verb, with every qualifier, its keywords and defaults, and an edit history to 2003 |
| ARCH | `rebuilding_starlet.txt`, next to the kits on the Freeware CD: how STARLET.L64 and LIB.L64 are built, and the `ARCH_DEFS` macros |
| VSI | VSI's release notes for BLISS V1.15-148 on OpenVMS x86-64, in the x86-64 cross-tools notes (<https://wasd.vsm.com.au/sys$common/syshlp/X86_XTOOLS-E0902-1_XGF4.RELEASE_NOTES>): how VSI retargeted the same dialect to another architecture |
| CS | DESIGN-0004 and the OpenVMS calling standard it follows, for the 64-bit mechanism array |
| probe | Not stated by any document; to be confirmed against BLISSA64 V1.11-7 in AXPbox once the oracle harness (PRD-0004 step 3) exists |
| vaxpunk | A choice of ours, where BLISS-64 is tied to Alpha |

The kit stores the release notes as text in variable-length records (a
16-bit length, the bytes, padded to even) with a 4-byte PCSI chunk header
(`04 82 3E 00`) every 15,876 bytes; dropping the headers and reading the
records gives the text back.

The Alpha BLISS kit carries no BLISS-64 manual. Its files are the two
compilers, the LRM and UM above (as PDF and PostScript, which is where
the font names in the kit come from; the kit's `BLSLREF.PDF`, carved out
with the PCSI chunk headers dropped, is byte for byte the 1987
`blslref.pdf`, so the LRM was never revised for Alpha), the release notes, `BLISS_AN.CLD`,
`BLI$CALLG.MAR`, two installation test programs and the library build
procedure: no help library. The release notes are the only DEC document of
the dialect: DEC's manuals stayed the two above, and the notes say how the
Alpha compilers differ from them. The online help is listed as missing in
the notes' known bugs.

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
- **`QUAD_LITERALS`.** BLISS-64EN has an `/ASSUME=[NO]QUAD_LITERALS`
  option, on by default, and a `[NO]QUAD_LITERALS` module switch, added
  in 1996. No document describes it. The likely reading is that with it
  off, literals are 32-bit values, sign-extended, as BLISS-32 code
  expects of `%X'FFFFFFFF'`; vbliss takes the default only until a probe
  says what `NOQUAD_LITERALS` does. (CLD; KIT; probe)
- **Arithmetic and comparisons** are the manual's at 64 bits: `+`, `-`,
  `*`, `/`, `MOD` are signed 64-bit operations, `LSS` and the rest signed,
  `LSSU` and the `U` forms unsigned, `LSSA` and the `A` forms address
  comparisons, which are unsigned: `-1 LSSA 0` is 0. The `[NO]OVERFLOW`
  switch turns overflow checking on and off; RN doesn't say which is the
  default, and BLISSA64 wraps without it, so `NOOVERFLOW` is. vbliss
  wraps and accepts the switch. (LRM 5.1; RN 2.26; oracle,
  `tests/bliss/arith.b64`; vaxpunk)

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

**VMS's 32-bit structures.** A field of a `$xxxDEF` block is a `FIELD`
item `[offset, position, size, extension]` used with `BLOCK[, BYTE]`, so
its size and extension are in the definition, not in the dialect: a
`PCB$L_STS` read through such a field is a 32-bit fetch in both
dialects, and only its extension decides the upper half in BLISS-64.
How DEC's `STARLET.R64` wrote the extension of `$L_` fields (0, or 1 for
the ones holding statuses) is not documented; vdefs writes 0, as the LRM's
BLISS-32 examples do, and a probe of `%FIELDEXPAND` on DEC's library
settles it. `EXTENSION` beyond that (the `E` of a field reference
`<P,S,E>`) is unchanged from the LRM. (LRM 11.2, 11.5, 11.10.3; probe; vaxpunk)

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
- **What the built-ins call.** The compiler's table pairs `SIGNAL`,
  `SIGNAL_STOP` and `SETUNWIND` with `LIB$SIGNAL`, `LIB$STOP` and
  `SYS$UNWIND`, as BLISS-32 did. (KIT)
- **`SIGNAL` takes a 32-bit condition.** The kit's messages include a
  diagnostic that `SIGNAL` passes only the low 32 bits of the condition
  value and that `SIGNALREF` should be used: a BLISS-64 condition value
  with bits set above 31 gets a message, and the rest of the arguments are
  passed as quadwords. (KIT; which compiler issues it and when, probe)
- **The handler's parameters** on OpenVMS are the LRM's three: the signal
  vector, the mechanism vector, and, for an `ENABLE`d handler, the enable
  vector, each by reference. On Alpha the signal vector is still the
  32-bit one (a longword count, then longwords), while the mechanism
  vector is the 64-bit `$CHFDEF` one, with quadword saved registers and a
  pointer to the 64-bit signal vector; a handler that set the routine's
  value through `CHF$L_MCH_SAVR0` on the VAX writes the quadword field
  instead. RN says nothing about this beyond Tru64's different
  parameters: it is the operating system's calling standard, not the
  dialect. (LRM 17.4.2; RN 2.33.1; CS)

  vaxpunk: the signal and mechanism arrays are DESIGN-0004's: the 32-bit
  signal array first, the 64-bit one through `CHF$PH_MCH_SIG64_ADDR`, the
  value in `CHF$IH_MCH_RETVAL`. vdefs gives BLISS-64 these from `$CHFDEF`.
- An `ENABLE`d handler is called through a jacket in the run-time library,
  `OTS$BLISS_STATIC_HANDLER` (and `OTS$BLISS_DYNAMIC_HANDLER` for
  `ESTABLISH`), which adds the enable vector to the arguments the system
  passes. (KIT: the names are in the compiler's table; how they are used,
  probe)

  vaxpunk: `ENABLE` writes the handler at `16(FP)` (DESIGN-0004
  *Condition handling*). The enable vector needs the same kind of jacket,
  which vbliss makes in each module that needs it (`vtools/docs/vbliss.md`,
  *Linkages and conditions*); `ESTABLISH` is a store to `16(FP)` and
  `REVERT` clears it. `tests/bliss/conditions.b64` continues, resignals and
  unwinds as OpenVMS does: the same depths, the same `SS$_UNWIND` calls.

## Built-ins

The VAX's machine built-ins are gone: no `MOVC3`, `INSQUE`, `MTPR`,
`PROBER`, `CALLG`, `EMUL`, `FFS`. Their replacements are the `CH$`
functions, the linkage functions, common built-ins like `ROT`, the Alpha
built-ins and the PAL built-ins below. (RN 2.3.1; KIT, where none of the
VAX names appears)

**Which need `BUILTIN`.** BLISSA64 takes `MAX`, `MIN`, `ABS`, `SIGN`,
`SIGNAL`, `SIGNAL_STOP`, `SETUNWIND` and the `CH$` functions as
predeclared, but `ACTUALCOUNT`, `ACTUALPARAMETER`, `NULLPARAMETER`,
`ARGPTR`, `ROT`, `SLL`, `SRL`, `SRA`, `ESTABLISH` and `REVERT` only once
a `BUILTIN` declaration names them: without one they are undeclared names
it calls as external routines. vbliss requires the declaration for the
same ones. An `ENABLE` actual that isn't `VOLATILE` gets a warning, as the
LRM's restriction says. (probe, `tests/bliss/builtins.b64` and
`conditions.b64`)

The list below is the compiler's own table (KIT), grouped. The last
column says when vbliss needs each: **now**, in PRD-0004; **later**, when
some code uses it; **no**, it means nothing on ARM64 (rejected, or
accepted as a no-op where the row says so).

| Built-in | What it does | Source | vbliss |
| --- | --- | --- | --- |
| `ACTUALCOUNT()` | number of actuals, from x9 | LRM 13.6 | now |
| `ACTUALPARAMETER(i)` | the i-th actual, from 1 | LRM 13.6 | now |
| `NULLPARAMETER(i)` | the i-th actual is absent or 0 | LRM 13.6 | now |
| `ARGPTR()` | a VAX-style copy of the argument list (*Linkage functions*) | RN 2.3.2 | now |
| `RETURNADDRESS()` | the caller's return address | KIT | later |
| `SIGNAL`, `SIGNAL_STOP`, `SETUNWIND` | condition handling | LRM 17 | now |
| `SIGNALREF`, `SIGNALREF_STOP` | the same, the condition a 64-bit value by reference | RN 2.33.3 | later |
| `ESTABLISH(rtn)`, `REVERT()` | set or clear the routine's handler at run time | RN 2.3.3 | now |
| `MAX`, `MIN`, `MAXU`, `MINU`, `MAXA`, `MINA`, `ABS`, `SIGN` | common arithmetic | LRM 5.2 | now |
| `%REF(e)` | the address of a temporary holding e, as an actual | LRM 5.2.2.3 | now |
| `CH$PTR`, `CH$PLUS`, `CH$DIFF`, `CH$RCHAR`, `CH$WCHAR` and their `_A`/`A_` forms | character pointers | LRM 20 | now |
| `CH$ALLOCATION`, `CH$SIZE` | buffer sizes, compile time | LRM 20 | now |
| `CH$MOVE`, `CH$COPY`, `CH$FILL` | move, concatenate and pad, fill | LRM 20 | now |
| `CH$EQL`, `CH$NEQ`, `CH$LSS`, `CH$LEQ`, `CH$GTR`, `CH$GEQ`, `CH$COMPARE` | string comparison with a fill character | LRM 20 | now |
| `CH$FIND_CH`, `CH$FIND_NOT_CH`, `CH$FIND_SUB`, `CH$FAIL` | search | LRM 20 | now |
| `CH$TRANSTABLE`, `CH$TRANSLATE` | translation | LRM 20 | later |
| `ROT(v, n)` | rotate | RN 2.9.6 | now: `ror` |
| `SLL`, `SRL`, `SRA` | shift one way by 0 to 63 | RN 2.9.5 | now |
| `UMULH(a, b)` | high half of the unsigned product | RN 2.9.6 | later: `umulh` |
| `CMPBGE`, `ZAP`, `ZAPNOT` | Alpha's byte-mask operations | RN 2.9.6 | later, a few instructions each |
| `TRAPB()`, `DRAINT()` | wait for pending arithmetic traps | RN 2.9.6; KIT | no: no-ops |
| `RPCC()` | the cycle counter | RN 2.9.6 | later: `CNTVCT_EL0` |
| `WRITE_MBX(a, v)` | store-conditional to an I/O mailbox | RN 2.9.6 | no |
| `BARRIER()` | memory barrier | RN 2.13 | now: `dmb ish` |
| `ADD_`, `AND_`, `OR_ATOMIC_LONG`/`_QUAD(p, e [, retries] [; old])` | atomic update, 1 if done within the retries; `p` naturally aligned | RN 2.9.3 | now: `ldxr`/`stxr` |
| `CMP_SWAP_LONG`, `CMP_SWAP_QUAD` | compare and swap | KIT; VSI | now |
| `CMP_STORE_LONG`, `CMP_STORE_QUAD(a, cmp, v, dest)` | compare at a, store v at dest if equal | RN 2.9.6 | later |
| `TESTBITSS`, `TESTBITSC`, `TESTBITCS`, `TESTBITCC` | test a bit and set or clear it, atomic against ASTs | RN 2.9.4 | now |
| `TESTBITSSI`, `TESTBITCCI` | the same, interlocked | RN 2.9.4 | now |
| `ADAWI(a, n)` | interlocked word add, returns faked VAX condition codes | RN 2.9.4 | later |
| `CALL_PAL(code, ...)` | any PAL call, code a compile-time constant | RN 2.9.2 | now |
| `PAL_INSQHIL`, `PAL_INSQTIL`, `PAL_REMQHIL`, `PAL_REMQTIL`, their `Q` (quadword) and `R` (resident) forms | interlocked queues | RN 2.9.2 | now, those the PAL has |
| `PAL_INSQUEL`, `PAL_INSQUEQ`, `PAL_REMQUEL`, `PAL_REMQUEQ`, their `_D` forms | non-interlocked queues | RN 2.9.2 | now, those the PAL has |
| `PAL_PROBER`, `PAL_PROBEW` | probe access for a mode | RN 2.9.2 | now |
| `PAL_MTPR_x`, `PAL_MFPR_x` (`IPL`, `ASTEN`, `ASTSR`, `SIRR`, `SISR`, `PCBB`, `WHAMI`, `TBIA`, `TBIS`...) | processor registers | RN 2.9.2; KIT | now, those the PAL has |
| `PAL_CHMK`, `PAL_CHME`, `PAL_CHMS`, `PAL_CHMU` | change mode | RN 2.9.2 | later |
| `PAL_HALT`, `PAL_BPT`, `PAL_BUGCHK`, `PAL_GENTRAP` | halt, breakpoint, bugcheck, software trap | RN 2.9.2 | later |
| `PAL_RD_PS`, `PAL_WR_PS_SW`, `PAL_SWASTEN`, `PAL_SWPCTX`, `PAL_RSCC`, `PAL_READ_UNQ`, `PAL_WRITE_UNQ`, `PAL_IMB`, `PAL_CFLUSH`, `PAL_DRAINA`, `PAL_LDQP`, `PAL_STQP` | the rest of OpenVMS's PAL | RN 2.9.2 | later, as the PAL has them |
| `ADDx`, `SUBx`, `MULx`, `DIVx`, `CMPx`, `CVTxy`, `CVTRxy` (`F`, `D`, `G`, `S`, `T`, with `L`, `Q`, `I`) | floating point | RN 2.10.1 | no, until vasm has floating point |

**PAL calls.** The `PAL_` prefix keeps them apart from the VAX built-ins
of the same names; inputs and outputs are the Alpha SRM's. RN and the
table differ at the edges: the table has `PAL_MFPR_ASN` where RN lists
`MTPR_ASN`, and `PAL_MTPR_PERFMON` last, added late. (RN 2.9.2; KIT)

vaxpunk: the PAL keeps Alpha's numbers (DESIGN-0001), so each `PAL_x`
built-in whose call vaxpunk implements is that call, and `CALL_PAL`
reaches vaxpunk's own (0x40 and up). One the PAL doesn't implement is a
compile error.

**Floating point** also brings `%S` and `%T` literals, `%FFLOAT` to
`%TFLOAT` to pass and return floating values by value, and
`ENVIRONMENT(NOFP)`. (RN 2.10, 2.11)

**Not built-ins.** The executables also hold GEM's intrinsics
(`EXCH_ATOMIC_x`, `INC_ATOMIC_x`, `MAX_ATOMIC_x`, `ENTER_CRITICAL`, the
`DE_` names...), the code generator's, shared with DEC's other GEM
compilers; they aren't in BLISS's built-in list, and vbliss doesn't take
them. (KIT)

## Lexical functions

New: `%HOST`, `%TARGET`, `%MODULE`, `%ROUTINE`, `%IDENT`, `%BLISS32E`,
`%BLISS64E`, `%BLISS32V`. (RN 2.12) Under `/A64`, `%BLISS(BLISS64E)` is 1
and `%BLISS(BLISS32)`, `%BLISS(BLISS32E)` and `%BLISS(BLISS64)` are 0;
`%BLISS64E(...)` expands to its actuals and `%BLISS16`, `%BLISS32`,
`%BLISS36` and `%BLISS32E` to nothing, while `%BLISS64` is no name at
all. `%BPVAL`, `%BPUNIT`, `%BPADDR` and `%UPVAL` are literal names, 64,
8, 64 and 8, which `%STRING` gives as their names. (probe) The kit's table has these too,
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

Module switches the 64-bit compiler knows, besides the manual's:
`OVERFLOW`, `ALPHA_REGISTER_MAPPING`, `BLOCK_ALIGNMENT`,
`DEFAULT_GRANULARITY`, `COUNT`, `LONG_DEFAULT`, `REF_LONG`,
`SIGNED_LONG`, `QUAD_LITERALS`, each with its `NO` form, and a `CHECK_x`
switch for each `/CHECK` option. The module head also takes `LANGUAGE`,
`ADDRESSING_MODE`, `ENVIRONMENT`, `OTS_LINKAGE` and `VERSION` besides
the manual's `IDENT`, `MAIN`, `OPTLEVEL` and the rest. A switch in the
module head beats the qualifier; a `SWITCHES` declaration beats both.
(RN 2.26, 2.8.5; KIT; UM 1.3.10)

The `BLISS` verb, as `BLISS_AN.CLD` defines it (CLD):

| Qualifier | Values, default first | Notes |
| --- | --- | --- |
| `/A32`, `/A64` | `/A32` | `/A64` runs BLISS-64EN |
| `/ASSUME=` | `NOALIAS`, `NOLONG_DEFAULT`, `QUAD_LITERALS`, `NOREF_LONG`, `NOSIGNED_LONG`, `BLOCK_ALIGNMENT=FULLWORD` | the `_LONG` ones and `QUAD_LITERALS` 64-bit only; `NOALIAS` is BLISS-32's `SAFE` (RN 2.19.1) |
| `/CHECK=` | `ALIGNMENT`, `FIELD`, `OPTIMIZE`, `SHARE` on; `ADDRESS_TAKEN`, `REDECLARE`, `PARAMETERS`, `LONGWORD`, `SHORT_ADDRESS` off; `ALL`, `NONE` | `LONGWORD` and `SHORT_ADDRESS` 64-bit only |
| `/[NO]COUNT` | `COUNT` | argument count in CALL linkages (RN 2.8.5) |
| `/[NO]INITIAL_PSECT` | on | `$INITIAL$` for `LOCAL` initial values, else `$PLIT$` (RN 2.19.10) |
| `/NAMES=` | `UPPERCASE` | external names without `EXTERNAL_NAME` |
| `/SYNTAX_LEVEL=` | 2 | reserved words (*The compiler*) |
| `/LANGUAGE=` | | `COMMON`, `BLISS16`, `BLISS32`, `BLISS36`, `BLISS32M`, `BLISS32E`, `BLISS64E`: warn where the source leaves the subset |
| `/INCLUDE=(dir,...)` | | directories for `REQUIRE` and `LIBRARY`; file types must then be written out (RN 2.19.14) |
| `/LIBRARY[=file]` | | compile a library instead of an object; not with `/OBJECT` |
| `/LIST[=file]`, `/SOURCE_LIST=`, `/[NO]MACHINE_CODE` | | *Listings* |
| `/TERMINAL=[NO]ERRORS` | `ERRORS` | BLISS-32's `STATISTICS` is gone |
| `/ERROR_LIMIT=n`, `/VARIANT=n` | 1 when given bare | `%VARIANT` |
| `/OBJECT`, `/DEBUG`, `/TRACEBACK`, `/[NO]CODE`, `/DIAGNOSTICS`, `/ANALYSIS_DATA` | | as BLISS-32 |
| `/CROSS_REFERENCE[=MULTIPLE]` | | in the CLD, though RN 2.19.15 lists it as dropped; probe |
| `/OPTIMIZE[=LEVEL=n,TUNE=x]`, `/ARCHITECTURE=`, `/GRANULARITY=`, `/ENVIRONMENT=[NO]FP`, `/TIE`, `/ALPHA_REGISTER_MAPPING`, `/ANNOTATIONS` | | Alpha code generation: vbliss accepts and ignores them |

Dropped from BLISS-32: `/DESIGN`, `/QUICK`, `/SOURCE_LIST=HEADER`, and
`/MACHINE_CODE_LIST`'s keywords. An output qualifier after an input file
puts the output next to it. (RN 2.19.15, 2.2.2)

The checks worth having in vbliss, because they find the bugs 64-bit
fullwords bring:

- `LONGWORD`: LONG without `SIGNED` or `UNSIGNED`; a 64-bit value stored
  in a 32-bit field or passed where a routine takes 32 bits. (RN 2.19.5)
- `SHORT_ADDRESS`: a fetch or store through an address held in less than
  64 bits, or an address stored in less than 64 bits. On vaxpunk, below
  2 GB, this is harmless until P2 space; off by default as in BLISSA64.
  (RN 2.19.7)
- `ADDRESS_TAKEN`: see ALIAS above; the dot lint's base.

## Libraries: REQUIRE and LIBRARY

`REQUIRE 'file'` reads source text in place, as the LRM says. `LIBRARY
'file'` loads a precompiled library: the declarations of a source
compiled with `/LIBRARY`, which writes no object. A library declares
names only; it holds no code or data, and only what it declares is
visible. (LRM 16.5, 16.6; CLD)

- **Per dialect.** A library is the compiler's internal tables, so it
  belongs to one compiler: BLISS-64EN reads `.L64` (and `.L64E`, `.LIB`),
  BLISS-32EN `.L32`. DEC's `STARLET.L64` was compiled from `STARLET.R64`
  with `BLISS/A64/LIBRARY`, `STARLET.L32` from `STARLET.REQ` with `/A32`.
  (RN 2.2.1, 5.1.1, chapter 4 item 3)
- **Search.** File name defaults are the user manual's; `/INCLUDE`
  adds directories. Where BLISSA64 looks for `LIBRARY 'SYS$LIBRARY:STARLET'`
  and for a bare name is a probe. (RN 2.19.14; LRM 16.6.3)

A library is not quite a `REQUIRE` done early. Only declarations are
allowed in its source: no `OWN`, `GLOBAL` or routine bodies, no
`GLOBAL LITERAL`, and `BIND` only to compile-time constants. Lexical
functions are evaluated when the library is compiled, not where it is
used, so `%VARIANT` or `%SWITCHES` inside it see the library's
compilation; `SWITCHES` inside it don't reach the user; and the names a
nested `LIBRARY` brought in are undeclared at the end, so they don't
leak. (LRM 16.6.2, 16.6.3)

vaxpunk: `.L64` is our own format (PRD-0004 *Non-goals*). Until compile
times say otherwise, vbliss may implement `LIBRARY` by reading the
matching `.R64` with the listing off, after checking the source holds
only what a library may and applying the rules above: evaluate its
lexical functions in a context of its own, keep its switches local, and
drop the nested libraries' names.

## Listings

What an oracle comparing BLISSA64's listing with vbliss's needs to know.

- **Turning it on.** `/LIST` writes a listing; `/SOURCE_LIST=` chooses
  the source part: `SOURCE` (default on), `EXPAND_MACROS` (each macro
  call followed by its expansion), `TRACE_MACROS` (each step of the
  expansion), `REQUIRE` and `LIBRARY` (the text of require files, and
  what libraries were read), `PAGE_SIZE=n`. The module switch forms are
  `LIST(SOURCE)`, `LIST(EXPAND)`, `LIST(TRACE)`, `LIST(REQUIRE)`,
  `LIST(LIBRARY)`, so a test can set them in the source. MACRO-32's
  `/SHOW=EXPANSIONS` is not a BLISS qualifier; PRD-0004 means
  `/SOURCE_LIST=EXPAND_MACROS`. (CLD; UM 1.3.7, 1.3.9)
- **Machine code.** `/MACHINE_CODE` (yes or no, default yes with
  `/LIST`) adds the generated code after each routine. On Alpha it is
  GEM's: Alpha instructions, the routine's procedure descriptor
  ("Register-Frame", "Stack-Frame" or "Null-Frame invocation
  descriptor", entry, saved registers, frame size, handler) and the
  linkage section. None of it compares with vbliss's ARM64 code; the
  oracle drops it. (RN 2.19.15; KIT)
- **What compares.** The source part, with expansions; compile-time
  output (`%PRINT`, `%INFORM`, `%WARN`, `%ERROR`); and the diagnostics,
  which appear after the line they're about with a severity and a
  `%BLS32-W-UNAVOLACC` style message (RN 2.7; the 64-bit compiler's
  facility name, `BLS64` in its strings, is a probe). Page headers (date, compiler version,
  page numbers) and the summary at the end (statistics, the command line)
  are dropped. BLISS-32's listing format (UM chapter 2) is the guide; how
  far BLISSA64's differs from it is a probe. (UM 2.2; KIT; probe)

vbliss: the source part with expansions in BLISSA64's layout, as closely
as the probes show, so the oracle compares text; the machine code part is
vbliss's own, the ARM64 each line became.

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

The require files themselves (`STARLET.R64`, `LIB.R64`, `STARLET.REQ`,
`LIB.REQ`, `ARCH_DEFS.REQ`, `CLIMAC.REQ`, `TPAMAC.REQ`) are not in the
BLISS kit, which holds no file of that name: they are in `SYS$LIBRARY`
of the OpenVMS Alpha system the oracle runs on, and the kit's install
procedure only compiles them into `.L32` and `.L64`. Reading them there
answers the questions about DEC's field definitions (probe 14) directly,
as reference only: vdefs takes nothing from them. (KIT; RN 5.1.1)

## To probe

What the sources leave open, each as a program for the oracle harness
(PRD-0004 step 3). `PRINT(x)` is the tiny print routine each side
provides, printing its argument as a signed 64-bit decimal and in hex; a
"listing" probe is answered by the `/LIST/SOURCE_LIST=EXPAND_MACROS`
output, compiled `/A64` unless it says `/A32`. The fragments go inside a
routine of a module.

1. **Sizes.** Are `%UPVAL` 8 and `%BPADDR` 64, and what does each
   allocation take?

   ```
   OWN S, V: VECTOR[3], B: BLOCK[2], BV: BITVECTOR[9], BB: BLOCKVECTOR[2,3];
   %PRINT(%UPVAL, ' ', %BPADDR, ' ', %ALLOCATION(S), ' ', %ALLOCATION(V), ' ',
          %ALLOCATION(B), ' ', %ALLOCATION(BV), ' ', %ALLOCATION(BB))
   ```

   Listing. Expected, if the defaults are as *Structures* reads them:
   `8 64 8 24 16 2 48`. **Answered:** as expected, and `BLOCK_BYTE[5]` is
   5; `%SIZE` gives 20 for `VECTOR[10, WORD]`, 8 for `REF VECTOR`, 3 for
   `BLOCK[3, BYTE]` and 6 for `BLOCKVECTOR[2, 3, BYTE]`.

2. **PLIT layout.** The count's size and value, item sizes, padding.

   ```
   BIND P = PLIT(1, BYTE(2), 3) : VECTOR[, BYTE];
   INCR I FROM -8 TO 17 DO PRINT(.P[.I]);
   ```

   Output: the count, as bytes, ahead of the data, and whether `3` is
   aligned after the byte. Again with `UPLIT` (no count) and under
   `LONG_DEFAULT`. **Answered** under `/A64`: items are packed with no
   alignment (`3` is at byte 9), the count is a quadword of the data's
   bytes in quadwords rounded up (3 here), and a string fills out its last
   unit with zeros (`PLIT('ABC', 'DEFGHIJKL')` takes 24 bytes). `INITIAL`
   packs the same way; on structured data its items are quadwords unless
   they say otherwise, with a warning when they overflow the data.

3. **`%ASCID`.** The descriptor's size and fields.

   ```
   BIND D = %ASCID 'HELLO' : VECTOR[, BYTE];
   INCR I FROM 0 TO 15 DO PRINT(.D[.I]);
   ```

   Output: an 8-byte BLISS-32 descriptor (length word, type 14, class 1,
   longword pointer) or a 64-bit one (`DSC64$`, with the -1 in the
   longword at 4). **Answered:** the 8-byte BLISS-32 descriptor.

4. **Extension.** What a fetch of -1 gives at each size.

   ```
   OWN L: LONG, LS: LONG SIGNED, W: WORD, BS: BYTE SIGNED;
   L = -1; LS = -1; W = -1; BS = -1;
   PRINT(.L); PRINT(.LS); PRINT(.W); PRINT(.BS); PRINT(.L<0,32,1>);
   ```

   Expected `4294967295 -1 65535 -1 -1`; the first is what vbliss's
   lint has to warn about. **Answered** by `tests/bliss/data.b64`: as
   expected, a LONG without SIGNED is zero-extended.

5. **ARGPTR.** The list for 3 actuals.

   ```
   ROUTINE R(A, B, C) = (LOCAL P: REF VECTOR; P = ARGPTR();
       INCR I FROM 0 TO 3 DO PRINT(.P[.I]); .P[0]);
   R(10, -1, 1^40)
   ```

   Output: count 3 in a quadword, the values whole, and what lies past
   the count.

6. **Linkage registers.** Which numbers are a register conflict?

   ```
   LINKAGE L_n = JSB(REGISTER = n);
   EXTERNAL ROUTINE X: L_n;
   ```

   One module per n from 0 to 31; the listing's diagnostics give the
   set BLISSA64 rejects. Informational only: vbliss takes VAX R0-R11.

7. **64-bit arithmetic edges.**

   ```
   PRINT(1^63); PRINT(1^64); PRINT(-1^-64);
   PRINT(-1 LSS 0); PRINT(-1 LSSU 0); PRINT(-1 LSSA 0);
   PRINT(%X'80000000' LSS 0);
   LOCAL N; N = 0;
   INCR I FROM %X'7FFFFFFFFFFFFFFE' TO %X'7FFFFFFFFFFFFFFF' DO
       (PRINT(.I); IF (N = .N + 1) GTR 3 THEN EXITLOOP);
   DECRU I FROM 1 TO 0 DO PRINT(.I);
   ```

   Output: whether `^` by 64 is 0 or undefined, the `A` comparisons'
   signedness, and whether `INCR` ending at the largest value stops.
   **In part** by `tests/bliss/arith.b64`: `1^63` is the sign bit and the
   `A` forms are unsigned.

8. **`QUAD_LITERALS`.** What `NOQUAD_LITERALS` changes.

   ```
   MODULE Q (MAIN = M, NOQUAD_LITERALS) = BEGIN
   ... PRINT(%X'FFFFFFFF'); PRINT(-1); PRINT(%X'100000000');
   ```

   Output, and the listing for a diagnostic on the last literal.

9. **Overflow.** Which is the default, `OVERFLOW` or `NOOVERFLOW`?

   ```
   LOCAL X; X = %X'7FFFFFFFFFFFFFFF'; PRINT(.X + 1);
   ```

   **Answered** by `tests/bliss/arith.b64`: it wraps, so `NOOVERFLOW`.

   Output: a wrapped value, or a signal.

10. **Predeclared structures.** The text of `VECTOR`, `BLOCK`,
    `BITVECTOR`, `BLOCKVECTOR` and `BLOCK_BYTE`.

    ```
    OWN V: VECTOR[2]; %PRINT(%FIELDEXPAND(V)) ! and a TRACE_MACROS listing of .V[1]
    ```

    Listing, if it shows them; failing that, `%SIZE` of each with and
    without a unit.

11. **The structure-formal bug.** Does BLISSA64 still miscompile it?

    ```
    STRUCTURE BAD[I, P, S] = [%UPVAL] (IF .I THEN BAD ELSE BAD + 8)<P, S>;
    OWN V: VECTOR[2] INITIAL(5, 7); BIND T = V: BAD;
    PRINT(.T[0, 0, 64]); PRINT(.T[1, 0, 64]);
    ```

    Output against the expected `7 5`, written as in RN chapter 4. If
    BLISSA64 is wrong, the oracle's expected output for such tests is
    hand-written. **Answered:** with `I` rather than `.I` (the access
    formal is a value), BLISSA64 V1.11-7 gives `7 5`; `tests/bliss/structs.b64`
    has the case.

12. **SIGNAL with a 64-bit condition.**

    ```
    SIGNAL(1^32 + 1); SIGNAL(1, 1^40);
    ```

    with a handler that prints `.SIG[0]`, `.SIG[1]`, `.SIG[2]` as
    longwords and the 64-bit array through the mechanism array. Listing:
    which statement gets the "lower 32 bits" diagnostic. Output: how
    LIB$SIGNAL receives quadword arguments.

13. **The mechanism array.** What a BLISS-64 handler sees.

    ```
    ROUTINE H(SIG: REF BLOCK[, BYTE], MCH: REF BLOCK[, BYTE], EN: REF VECTOR) =
      (PRINT(.MCH[CHF$IS_MCH_ARGS]); PRINT(.MCH[CHF$IS_MCH_DEPTH]);
       MCH[CHF$IH_MCH_SAVR0] = 42; SETUNWIND(); 0);
    ```

    (with `LIBRARY 'SYS$LIBRARY:STARLET'`) enabled in a routine that
    signals; the routine's caller prints the
    value it gets back (42 if the unwind returns the saved R0). Also
    prints `.EN[0]` and the first enable actual to check the enable
    vector.

14. **STARLET's longword fields.** How DEC's `STARLET.L64` defines them.

    ```
    LIBRARY 'SYS$LIBRARY:STARLET';
    %PRINT(%FIELDEXPAND(CHF$L_SIG_NAME), ' ', %FIELDEXPAND(PCB$L_STS))
    ```

    Listing: the extension DEC wrote for `$L_` fields, which vdefs
    follows. Also shows where `LIBRARY` finds `STARLET`. Reading
    `SYS$LIBRARY:STARLET.R64` on the oracle's disk answers the first
    part without compiling.

15. **Libraries.** That a library's lexical functions are fixed when it is
    compiled.

    ```
    ! LV.R64:  LITERAL V = %VARIANT;
    ! compiled BLISS/A64/LIBRARY/VARIANT=3 LV.R64, then:
    LIBRARY 'LV'; PRINT(V);   ! compiled /VARIANT=5
    ```

    Expected `3`.

16. **The listing itself.** One module with a macro, a `REQUIRE`, a
    `%PRINT`, a warning and a routine, listed `/SOURCE_LIST=(EXPAND_MACROS,
    REQUIRE)/MACHINE_CODE` and `/NOMACHINE_CODE`. Answers the layout
    (line numbers, nesting columns, how expansions and require text are
    marked, where diagnostics go, the message facility name) and whether
    `/CROSS_REFERENCE` is accepted.
    **Answered** for the source part by `tests/bliss/macros.b64` and
    `lexical.b64`; `vtools/docs/vbliss.md` (*The listing*) has the layout.
    The facility is `BLS64`; a source line wider than about 132
    characters doesn't survive the oracle's typing it in, so test sources
    keep to 120.

17. **Under `/A32`:** items 1 to 5 again, and what a BLISS-32 routine
    receives from a BLISS-64 caller passing `1^32 + 5` and `-1`.
