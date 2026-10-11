+++
title = "BLISS compiler"
summary = "DEC's systems language; BLISS-64 here, bootstrapped in Rust, to self-host"
aka = ["BLISS", "BLISS-64", "BLISS-32", "BLISS-64EN", "BLISS-32EN", "BLISSA64", "BLISS.EXE", "vbliss", ".B64", ".B32", ".R64", ".REQ"]
kind = "layered"
intent = "goal"
research = "done"
refs = ["docs/prd/0004-bliss64-compiler.md", "crosstools/vtools/docs/vbliss.md", "crosstools/vtools/docs/bliss64.md", "crosstools/vtools/crates/vbliss", "crosstools/vtools/bliss", "vms/exec/numtim.b64", "vms/sysexe/tpu"]

[[feature]]
id = "stage0"
title = "Stage 0: vbliss compiles BLISS-64 on the host, checked against BLISSA64"
status = "done"
needs = ["/host/vbliss"]
note = "PRD-0004 steps 3-6 and 8"

[[feature]]
id = "definitions"
title = "LIB and STARLET require files made from the macro libraries"
status = "done"
needs = ["/host/vdefs"]
note = "PRD-0004 step 7"

[[feature]]
id = "bliss32"
title = "BLISS-32 (/A32) and the LONG_DEFAULT, REF_LONG and SIGNED_LONG switches"
status = "done"
note = "PRD-0004 step 9"

[[feature]]
id = "dot-lint"
title = "The dot lint, rules 1 to 4"
status = "done"
note = "PRD-0004 step 10; recall 37%, no false positives on the V4.3 sources"

[[feature]]
id = "pilot"
title = "The pilot: $NUMTIM in BLISS-64, in the executive"
status = "done"
milestone = "m1"
note = "PRD-0004 step 11; PRD-0003 tier 1 #7; vms/exec/numtim.b64"

[[feature]]
id = "in-use"
title = "Programs in BLISS-64: TPU and EVE, NSLOOKUP, the TCPIP utility, the DNS resolver"
status = "done"
note = "PRD-0006; vms/sysexe/tpu, tcpip$nslookup.b64, tcpip.b64, lib/dns.b64"

[[feature]]
id = "vasm-port"
title = "VASM: vasm in BLISS-64, writing the same objects as the Rust vasm"
status = "done"
needs = ["/host/vrun"]
note = "PRD-0004 steps 12-13"

[[feature]]
id = "missing-constructs"
title = "GLOBAL BIND, GLOBAL registers, default structure references, PAL and atomic built-ins"
status = "todo"
note = "vbliss.md: not yet"

[[feature]]
id = "lint-5-6"
title = "Dot lint rules 5 and 6: stored addresses, formals passed the other way"
status = "todo"
note = "PRD-0004 The dot lint"

[[feature]]
id = "stage1"
title = "Stage 1: BLISS.EXE, the compiler in BLISS-64, with VASM linked in"
status = "todo"
needs = ["#vasm-port"]
note = "PRD-0004 step 14"

[[feature]]
id = "self-hosting"
title = "Stages 2 and 3 identical, the build on stage 1, stage 0 frozen"
status = "todo"
needs = ["#stage1"]
note = "PRD-0004 step 15"

[[feature]]
id = "native"
title = "BLISS at the DCL prompt on vaxpunk writes the same object as the host"
status = "todo"
needs = ["#self-hosting", "/rms/core", "/dcl/cld"]
note = "PRD-0004 step 16"

[[feature]]
id = "native-link"
title = "LINK and LIBRARY on vaxpunk, so a program is built there end to end"
status = "todo"
needs = ["#native"]
note = "PRD-0004 non-goal and open question: vlink and vlib stay on the host until a PRD ports them, as LINK (t8y:/utils/linker) and LIBRARY (t8y:/utils/librarian)"

[[feature]]
id = "libraries"
title = "Precompiled libraries (.L64) for STARLET and LIB"
status = "todo"
note = "PRD-0004 open question"

[[feature]]
id = "debug-symbols"
title = "Debug symbol records for DEBUG"
status = "todo"
note = "PRD-0004 non-goal; the listing is the debugging aid"

[[needs]]
on = "/host/vbliss"
why = "Stage 0, and the bootstrap after it"

[[needs]]
on = "/exec/calling-standard"
why = "Its routines follow DESIGN-0004"

[[needs]]
on = "/rtl/librtl"
why = "SIGNAL and SIGNAL_STOP call LIB$SIGNAL and LIB$STOP"
+++

BLISS is DEC's systems programming language: typeless, built of
expressions, with a name standing for its address and `.X` for its
value, a strong macro language, and `LINKAGE` declarations that let it
call assembly routines with their register contracts. DEC wrote much of
the VMS executive and most utilities in it. BLISS-32 compiled for the
VAX; on Alpha, BLISS-32EN and BLISS-64EN (`BLISS/A32`, `/A64`) ran on
GEM. DEC put the Alpha compilers on the Freeware CD, and VSI ships them
for x86-64.

vaxpunk's BLISS is BLISS-64 retargeted to ARM64
([PRD-0004](../../prd/0004-bliss64-compiler.md)). Stage 0, `vbliss`,
is in Rust (`t8y:/host/vbliss`) and throwaway. It compiles BLISS-64 and
BLISS-32, is checked against DEC's BLISSA64 under AXPbox for listings
and behaviour, reads require files that `vdefs` makes from the macro
libraries, has the dot lint, and generates code through a QBE-style IR
and a simple ARM64 back end, with no LLVM. It is the system's BLISS
compiler: `numtim.b64` is in the executive, and TPU, NSLOOKUP and the
DNS resolver are BLISS-64. VASM, vasm rewritten in BLISS-64, is done.
Next come stage 1, `BLISS.EXE` in BLISS-64, then stages 2 and 3
identical, then `BLISS` at the DCL prompt. Linking on vaxpunk waits for
a PRD of its own.
