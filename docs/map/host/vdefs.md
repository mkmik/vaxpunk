+++
title = "vdefs (BLISS require files)"
summary = "Writes STARLET.R64 and LIB.R64 from the macro libraries"
aka = ["vdefs", "STARLET.R64", "LIB.R64", "STARLET.REQ", "LIB.REQ", "services.txt", "just defs"]
kind = "tool"
intent = "goal"
research = "done"
refs = ["docs/prd/0004-bliss64-compiler.md", "crosstools/vtools/crates/vdefs", "crosstools/vtools/lib"]

[[feature]]
id = "require-files"
title = "lib.r64, starlet.r64, lib.req and starlet.req from lib.mlb and starlet.mlb"
status = "done"
note = "PRD-0004 step 7, PR #102"

[[feature]]
id = "services"
title = "services.txt: each system service's argument mechanisms, for the dot lint"
status = "done"
note = "PRD-0004 step 10"

[[feature]]
id = "stale-check"
title = "Its test fails when the files are stale"
status = "done"

[[feature]]
id = "service-macros"
title = "System service macros: $QIOW(CHAN = ..., FUNC = ...) and the rest, keyword macros calling SYS$name, with each service's EXTERNAL ROUTINE"
status = "done"
note = "UM 8.3: a keyword macro for each of starlet.mlb's 92 $name_S macros, its formals and defaults, declaring the service in a block of its own; a required keyword left out is an error (the language test's service_macros)"

[[feature]]
id = "rms-macros"
title = "RMS control block macros: $FAB, $RAB, $NAM and the XABs, with their _INIT and _DECL forms"
status = "done"
note = "UM 8.4: crates/vdefs/src/starlet.bli, written for vaxpunk: $FAB, $RAB, $NAM and the XABs with their _INIT and _DECL forms, the keywords of the MACRO-32 macros plus FNM and DNM, and the RMS services with ERR and SUC. $XABKEY's key of reference is KREF, as REF is reserved (the language test's rms_and_helper_macros)"

[[feature]]
id = "helper-macros"
title = "STARLET's helper macros: $BBLOCK, $DESCRIPTOR, $BITPOSITION, $FIELDWIDTH, $FIELDMASK, $EQULST, $SHR_MSGDEF"
status = "done"
note = "starlet.bli: $BBLOCK, $DESCRIPTOR (a %ASCID), $BYTEOFFSET, $BITPOSITION, $FIELDWIDTH, $EXTENSION and $FIELDMASK, which also take a $V_ bit number, $EQULST and $SHR_MSGDEF. vaxpunk's STARLET has no SHR$_ codes yet for $SHR_MSGDEF to add to"

[[feature]]
id = "vax-builtin-macros"
title = "Macros for the VAX built-ins: FFS, FFC, MOVC3, MOVC5, CMPC3, LOCC, SKPC, SPANC, EMUL, EDIV, ASHQ, INSQUE, REMQUE, INSQHI, MTPR, MFPR, CALLG and the rest"
status = "done"
note = "crates/vdefs/src/lib.bli, in LIB: 43 built-ins, each a macro that in BUILTIN X marks the block and elsewhere is $X, by CH$ functions, inline BLISS or PAL calls (tests/bliss/vaxbi32, vaxbi64; the language test's vax_builtin_macros). The V4.3 corpus compiles 50 modules, from 30. Left out: the decimal string ones and CRC; BUGW and BUGL wait for t8y:/platform/pal#bugchk; CALLG, and EMUL and EDIV under BLISS-32, call LIB$CALLG, LIB$EMUL and LIB$EDIV, which t8y:/rtl/librtl hasn't yet"

[[feature]]
id = "other-requires"
title = "TPAMAC.REQ for LIB$TPARSE's state tables, CLIMAC.REQ and ARCH_DEFS.REQ"
status = "done"
note = "lib/tpamac.req, arch_defs.req and climac.req, written for vaxpunk, and $TPADEF in starlet.mlb. TPAMAC makes VAX-format tables of 32-bit addresses for a LIB$TPARSE that t8y:/rtl/librtl doesn't have yet; ARCH_DEFS says Alpha, and ARM64; CLIMAC's request codes went from STARLET with OpenVMS Alpha, so a program declares them (the language test's other_requires)"

[[feature]]
id = "field-signs"
title = "Field signs as DEC's STARLET and LIB have them: CHF$L_SIG_NAME is 4,0,32,1, PCB$L_STS 128,0,32,0"
status = "done"
note = "bliss64.md probe 14: SDL marks links, addresses, condition values and times signed, and the macro libraries don't say. vdefs's SIGNED table names the 75 fields of vaxpunk's libraries DEC's sign-extend; vaxpunk's 32-bit addresses are below 2 GB, so the sign changes no address"
+++

On VMS, definitions such as `$PCBDEF` or `SS$_NORMAL` come from one
source per structure in SDL (`t8y:/lang/sdl`), which writes a macro library
for MACRO, a require file for BLISS and headers for the other languages.
vaxpunk's single source is its macro libraries, so `vdefs` reads
`lib.mlb` and `starlet.mlb` and writes the BLISS require files from
them. A BLISS-64 program then reads a field of `$PCBDEF` at the offset
MACRO-32 does. `just defs` regenerates them, and the test catches a
library edited without them.

DEC's STARLET and LIB require files hold more than definitions: a
keyword macro for every system service, the RMS control block macros,
helper macros, and macros that stand in for the VAX built-ins BLISS
on Alpha dropped. vdefs writes the service macros from the `$name_S`
macros and copies in the rest, written for vaxpunk in
`crates/vdefs/src/*.bli`.
