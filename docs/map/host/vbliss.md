+++
title = "vbliss (BLISS-64 bootstrap compiler)"
summary = "Stage 0 of the BLISS-64 compiler, in Rust, throwaway once BLISS compiles itself"
aka = ["vbliss", "stage 0", "BLISS-64", "BLISS-32", "/A64", "/A32", "vbliss.md", "bliss64.md"]
kind = "tool"
intent = "goal"
research = "done"
refs = ["docs/prd/0004-bliss64-compiler.md", "crosstools/vtools/crates/vbliss", "crosstools/vtools/docs/vbliss.md", "crosstools/vtools/docs/bliss64.md", "crosstools/ods/vms/bliss-oracle.py"]

[[feature]]
id = "core"
title = "Routines, data, expressions and control; a small IR and an ARM64 back end"
status = "done"
note = "PRD-0004 steps 3 and 4, PR #97"

[[feature]]
id = "macros"
title = "Macros, lexical functions, REQUIRE, the listing with expansions"
status = "done"
note = "PRD-0004 step 5"

[[feature]]
id = "structures"
title = "STRUCTURE, BLOCK, VECTOR, FIELD, BIND, PLIT"
status = "done"
note = "PRD-0004 step 6"

[[feature]]
id = "linkages"
title = "LINKAGE, BUILTIN, ENABLE, SIGNAL"
status = "done"
note = "PRD-0004 step 8"

[[feature]]
id = "bliss32"
title = "/A32 and the _LONG switches"
status = "done"
note = "PRD-0004 step 9"

[[feature]]
id = "dot-lint"
title = "The dot lint, rules 1 to 6"
status = "done"
note = "PRD-0004 step 10; rules 5 and 6 with t8y:/lang/bliss#lint-5-6"

[[feature]]
id = "missing"
title = "GLOBAL BIND, GLOBAL registers, default structure references, PAL and atomic built-ins"
status = "done"
note = "t8y:/lang/bliss#missing-constructs"

[[feature]]
id = "psects"
title = "PSECT attributes honoured, PLITs in the psect asked for, directories in REQUIRE"
status = "done"
note = "and EXTERNAL_NAME; crates/vbliss/tests/asm.rs"

[[feature]]
id = "floating-point"
title = "Floating point: %E, %D, %G, %S and %T literals, the ADDx ... CVTxy built-ins, %FFLOAT to %TFLOAT, ENVIRONMENT(NOFP)"
status = "done"
needs = ["/host/vasm#fp", "/exec/calling-standard#floating-point"]
note = "RN 2.10, 2.11: literals as BLISSA64 encodes them (oracle FLTLIT2, FLTLIT3; D at G's precision), with its overflow and underflow warnings; the ADDx ... CVTRxy built-ins on operands in memory, VAX formats through IEEE double as Alpha does D through G, SUBx and DIVx in VAX operand order (tests/bliss/fltops, BLISSA64's output); %xFLOAT actuals and results by DESIGN-0004; ENVIRONMENT(NOFP)'s warning. BLISSA64 gives %E'-1.5' a stray low bit, vbliss the exact value"

[[feature]]
id = "general-calls"
title = "General routine calls, LINKAGE(address, actuals), and empty actuals, R(1, , 3)"
status = "done"
note = "LRM 12.1, 12.2: L(.P, 5) calls .P by L, and an actual left out passes 0, as DEC's STARLET.R64 service macros leave one ($ASSIGN); tests/language.rs"

[[feature]]
id = "linkage-types"
title = "INTERRUPT and EXCEPTION linkages; the COUNT and NOCOUNT linkage options, module switch and /[NO]COUNT"
status = "done"
note = "RN 2.8.4, 2.8.5: an INTERRUPT or EXCEPTION routine takes its formals from the stack, pops the PAL's parameters and returns with REI, and can't be called; NOCOUNT, the linkage option, module switch or /NOCOUNT, passes no count in x9, and its routines can't read their argument lists"

[[feature]]
id = "predeclared-linkages"
title = "The predeclared linkages BLISS, FORTRAN, FORTRAN_SUB and FORTRAN_FUNC, and the LINKAGE switch's default linkage"
status = "done"
note = "LRM 12.1.3, 13.5: BLISS and the FORTRAN linkages are the calling standard; SWITCHES LINKAGE(name) gives routines declared after it, and calls through an address, its linkage. The module's names are a block inside the predeclared ones, which it may declare again"

[[feature]]
id = "routine-attributes"
title = "Routine attributes: ADDRESSING_MODE, the value's unit, sign, FIELD or [REF] structure, prototypes on EXTERNAL and FORWARD ROUTINE, VARIABLE"
status = "done"
note = "RN 2.21, 2.29, 2.19.8: ADDRESSING_MODE ignored; a value's unit, sign or structure is accepted and, as in BLISSA64 (oracle RATTR), changes no code; prototypes and VARIABLE accepted, %BLISS(PROTOTYPES) 1. Checking actuals against formals is /CHECK=PARAMETERS's (#checks)"

[[feature]]
id = "data-attributes"
title = "Data attributes: RANGE on literals, EXTERNAL_NAME on literals, GRANULARITY, REF VOLATILE and REF ALIGN structures, VOLATILE and NOCHECK_ALIGNMENT fields"
status = "done"
note = "LRM 9.12; RN 2.14, 2.16.1, 2.25, 2.28: literal ranges checked (message 036), EXTERNAL_NAME on GLOBAL and EXTERNAL LITERAL, GRANULARITY ignored as ARM64 writes bytes, VOLATILE and ALIGN between REF and the structure, VOLATILE and NOCHECK_ALIGNMENT fields"

[[feature]]
id = "forward-data"
title = "FORWARD data declarations, and a name in its own INITIAL"
status = "done"
note = "LRM 10.3: FORWARD Y, then OWN or GLOBAL Y (message 086 if never), and an OWN or GLOBAL is its own address in its INITIAL, as a queue head is (F11X and RTPAD in V4.3)"

[[feature]]
id = "psect-classes"
title = "PSECT LINK = and PSECT INITIAL =, and /[NO]INITIAL_PSECT"
status = "done"
note = "RN 2.19.10, 2.22: accepted; nothing goes in either on ARM64, which has no linkage section and makes a LOCAL's INITIAL code. /[NO]INITIAL_PSECT is #qualifiers'"

[[feature]]
id = "codecomment"
title = "CODECOMMENT 'text', ... : block, its text in the machine code listing"
status = "done"
note = "LRM 4.10: the texts are comments in the code, `comment` in the IR, ahead of the block's code; the machine code listing (#listing) will show them"

[[feature]]
id = "register-builtins"
title = "BUILTIN R0 ... R11: registers as built-ins, named as the linkages name them"
status = "todo"
note = "RN 2.9.1: Alpha BLISS reads registers with the VAX syntax; vbliss says R0 is not a built-in it knows. AP, FP, SP and PC stay errors, as on Alpha"

[[feature]]
id = "lexical-functions"
title = "%HOST, %TARGET and %BLISS32V; %RAD50_11 and %P under /A32"
status = "done"
note = "RN 2.12, oracle HOST1-4: %HOST and %TARGET are 1 for ARM64 and VMS, as vaxpunk is; %BLISS32V is 0; %IDENT, %MODULE and %ROUTINE are strings; %BLISS(PROTOTYPES) is 1. %RAD50_11 and %P, which BLISSA64 has under /A64 too (oracle RAD64)"

[[feature]]
id = "macro-edges"
title = "Macros as BLISSA64 reads DEC's: formals never expanded, KEYWORDMACRO without formals, %STRING of a keyword, empty macro actuals"
status = "done"
note = "Oracle MACCNT, STRKEY, KWNOF: empty actuals at the end of a macro call don't count, so $RMS_BITS and $RMS_VALPR2 recurse to an end; %STRING takes reserved words; a KEYWORDMACRO without formals is a simple macro. A macro name as a formal is expanded, as LRM 15.2.1's name-quote level says and BLISSA64 does (oracle FORMAC): SORT32's clash was a library's to avoid"

[[feature]]
id = "declaration-edges"
title = "Declarations as BLISSA64 takes them: a stray ; between declarations, BUILTIN of a name already BUILTIN"
status = "done"
note = "Oracle STRAY, BLTWICE: a stray ; between a module's declarations; BUILTIN of a name already BUILTIN in the block"

[[feature]]
id = "source-text"
title = "Sources in DEC MCS, Latin-1, not only UTF-8"
status = "done"
note = "vbliss::read_source reads a byte a character, and a string's bytes are the file's; DEC's STARLET.R64 has a copyright sign, and 5 V4.3 modules other 8-bit characters"

[[feature]]
id = "library-semantics"
title = "LIBRARY as LRM 16.6 has it: lexical functions and switches fixed when the library is compiled, nested libraries' names kept out, /LIBRARY for sources of declarations only"
status = "done"
note = "LRM 16.6: a library's source is read in a context of its own, which sees the predeclared names and its own, with declarations only (message 200); the module gets the names it declares, not those its own libraries gave it; a library read twice gives the same names. %VARIANT in a library is the module's until probe 15 says otherwise; the .L64 format is t8y:/lang/bliss#libraries"

[[feature]]
id = "switches"
title = "Switches that act: %SWITCHES and SWITCHES scopes, LIST, ERRS, NOCODE, OVERFLOW checking, QUAD_LITERALS"
status = "done"
note = "LRM 18.2, RN 2.26: on-off switches are kept block by block, %SWITCHES sees them (oracle SWIT); NOCODE makes no object; OVERFLOW checks a routine's +, - and * and signals SS$_INTOVF (BLISSA64's GEM crashes on it); NOQUAD_LITERALS sign-extends as BLISSA64 (oracle QLIT). LIST's options are #listing's, ERRS #qualifiers'"

[[feature]]
id = "pal-builtins"
title = "The other PAL_ built-ins, as the PAL gains their calls: interlocked queues, IPIR and WHAMI, MCES, FEN"
status = "todo"
needs = ["/platform/pal#interlocked-queues", "/platform/pal#smp-calls", "/platform/pal#machine-checks", "/platform/pal#floating-point"]
note = "bliss64.md Built-ins: one the PAL lacks is a compile error. CMP_STORE_x can't be one store-exclusive on ARM64 and WRITE_MBX means nothing here"

[[feature]]
id = "qualifiers"
title = "The BLISS verb's other qualifiers: /SOURCE_LIST, /MACHINE_CODE, /NAMES, /SYNTAX_LEVEL, /TERMINAL, /ERROR_LIMIT, /LANGUAGE, /DEBUG, /TRACEBACK, /[NO]CODE, Alpha's code generation ones ignored"
status = "todo"
note = "BLISS_AN.CLD, bliss64.md Switches and qualifiers: today /A32, /A64, /ASSUME, /VARIANT, /INCLUDE, /OBJECT and /LIST, any other a usage error. /SYNTAX_LEVEL=2 reserves QUAD and ALIAS, which are names today. /DIAGNOSTICS and /ANALYSIS_DATA write nothing until t8y:/layered/lse and t8y:/layered/sca"

[[feature]]
id = "checks"
title = "/CHECK and the CHECK_x switches: LONGWORD, SHORT_ADDRESS, ADDRESS_TAKEN, ALIGNMENT, FIELD, SHARE, REDECLARE, PARAMETERS; LANGUAGE's transportability checks"
status = "todo"
note = "RN 2.19.2-2.19.8, 2.26; LRM appendix C. bliss64.md names LONGWORD, SHORT_ADDRESS and ADDRESS_TAKEN as the ones that find 64-bit bugs; LANGUAGE(COMMON) is accepted and checks nothing"

[[feature]]
id = "listing"
title = "The rest of the listing: /MACHINE_CODE's ARM64, /SOURCE_LIST and LIST options, %TITLE and %SBTTL headers, the closing summary"
status = "todo"
note = "PRD-0004 Listing; the listing is the source part with expansions, whatever LIST says, under a 'Source Listing' header"

[[feature]]
id = "diagnostics"
title = "BLISS's messages: the user manual's 250 texts and severities, going on past the first error, the Alpha informationals"
status = "todo"
note = "vbliss writes BLISSA64's text for the 8 messages the tests meet, its own for the rest, and stops at the first error. Missing too: numeric literal overflow, SIGNAL's low 32 bits, names over 64 characters, a NOVALUE routine's value used. An up-level reference to an outer routine's LOCAL gets message 103, Illegal up-level reference"

[[feature]]
id = "probes"
title = "The open probes of bliss64.md answered by the oracle: 5, 7, 8, 10, 12, 13, 15, 17"
status = "wip"
needs = ["/host/axpbox-oracle#bliss"]
note = "5, 7, 8, 12, 13 and 15 answered (bliss64.md), with tests/bliss programs whose listings and output are BLISSA64's: argptr5, edges7, qlit, and the language probes kwnof, stray, bltwice, gencall, pscls, blksemi, lint3, host2-4, swit, fltops, fltlit3. Left: 10, the predeclared structures' text, and 17, /A32 against /A64"

[[feature]]
id = "dec-libraries"
title = "DEC's STARLET, LIB, TPAMAC and CLIMAC require files compile and expand as under BLISSA64"
status = "todo"
needs = ["#source-text", "#lexical-functions", "#predeclared-linkages", "#routine-attributes", "#general-calls", "#macro-edges", "#declaration-edges"]
note = "A local check, as the corpus test is: read from the oracle's SYS$LIBRARY, never committed. STARLET.R64 stops at line 26628 today. Patched by hand past those gaps, DEC's Alpha STARLET.REQ and LIB.REQ let 24 of the 1,260 V4.3 modules compile, against 19 with vaxpunk's; most of the rest need DEC's generated libraries or VAX built-ins"

[[feature]]
id = "debug-symbols"
title = "Debug symbol table records, for a future DEBUG"
status = "todo"
needs = ["/host/vms-obj#debug-symbols"]
note = "PRD-0004 non-goal: the listing is the debugging aid"

[[feature]]
id = "frozen"
title = "Frozen once stage 1 replaces it: bug fixes only, the bootstrap's first stage"
status = "todo"
note = "PRD-0004 step 15; t8y:/lang/bliss stage1 and self-hosting"

[[needs]]
on = "/host/vasm"
why = "Writes vasm assembly, which vasm assembles"

[[needs]]
on = "/host/vdefs"
why = "STARLET.R64 and LIB.R64, and services.txt for the dot lint"

[[needs]]
on = "/exec/calling-standard"
why = "BLISS-64's default linkage is the standard"
soft = true

[[needs]]
on = "/host/axpbox-oracle"
why = "BLISSA64's listings and behaviour are the expected outputs"
soft = true
+++

`vbliss` is the BLISS-64 compiler's stage 0, written in Rust: the
compiler that compiles the real one ([PRD-0004](../../prd/0004-bliss64-compiler.md)).
It accepts the dialect DEC's BLISS-64EN accepted on OpenVMS Alpha, and
BLISS-32 with `/A32`, and writes vasm assembly through a small IR, with
no optimizer and no LLVM. DEC's BLISSA64 in AXPbox is the oracle for
listings and behaviour.

It is the system's BLISS-64 compiler today: `vms/build.rs` compiles
`$NUMTIM`, TPU and EVE, the DNS resolver and other programs with it, and
fails on any warning. It is throwaway by design.

It doesn't yet take everything BLISSA64 takes. Its todo features come
from checking it against the LRM, the release notes and
`BLISS_AN.CLD`, one probe program per construct, and from compiling
DEC's own require files from the oracle's `SYS$LIBRARY` and the V4.3
sources against them, locally. The biggest are floating point, general
routine calls, the attributes and linkages DEC's STARLET leans on, real
`LIBRARY` semantics, switches that do something, and the compiler's
interface: qualifiers, listing and messages. They land here first, so
stage 1 ports a compiler whose extent is known. Stage 1, the compiler
written in BLISS-64, and the self-hosting stages belong to
`t8y:/lang/bliss`; once they pass, stage 0 is frozen and only starts the
bootstrap.
