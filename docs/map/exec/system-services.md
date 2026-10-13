+++
title = "System service dispatch"
summary = "SYS$ entry points in a user-readable vector, CHMK and CHME into the executive"
aka = ["SYS$name", "system service vector", "SYS$PUBLIC_VECTORS", "EXE$CMODKRNL", "EXE$CMODEXEC", "CHMK", "CHME", "$CMKRNL", "$CMEXEC", "SYS.STB", "SS$_ILLSER", "SS$_ARG_GTR_32_BITS", "IFNORD", "IFNOWRT", "user-written system services", "privileged shareable image", "PLV", "SSLOG"]
kind = "core"
intent = "goal"
research = "done"
milestone = "m0"
refs = ["docs/design/0002-executive-processes.md", "docs/design/0004-calling-standard.md", "docs/adr/0023-calling-standard.md", "docs/prd/0005-macro32-on-the-calling-standard.md", "vms/exec/syssrv.mar", "vms/exec/sysunwind.mar", "crosstools/vtools/lib/starlet.mlb", "crosstools/vtools/lib/services.txt"]

[[feature]]
id = "vector"
title = "SYS$name in a user-readable vector page; CHMK to EXE$CMODKRNL, which calls EXE$name"
status = "done"
milestone = "m0"
note = "DESIGN-0002 System services; syssrv.mar"

[[feature]]
id = "cmexec"
title = "$CMKRNL, and $CMEXEC through CHME to EXE$CMODEXEC, with CMKRNL or CMEXEC"
status = "done"
milestone = "m0"
note = "Privilege checks from PRD-0003 step 1"

[[feature]]
id = "probes"
title = "Arguments probed in the caller's mode: IFNORD, IFNOWRT, EXE$PROBER_DSC, SS$_ACCVIO"
status = "done"
milestone = "m0"
note = "DESIGN-0002 System services"

[[feature]]
id = "standard"
title = "On the calling standard: arguments in x0-x7 and the stack, count in x9, SS$_ARG_GTR_32_BITS"
status = "done"
needs = ["/exec/calling-standard#services"]
note = "PRD-0005 step 6, ADR-0023 decision 9"

[[feature]]
id = "sys-stb"
title = "SYS.STB from EXEC.EXE's map, which programs link against"
status = "done"
milestone = "m0"
note = "vms/build.rs; DESIGN-0002"

[[feature]]
id = "starlet"
title = "The $name_S and $name_G macros and the STARLET definitions, MACRO-32 and BLISS"
status = "done"
note = "starlet.mlb; starlet.req and starlet.r64 from vdefs"

[[feature]]
id = "all-services"
title = "Every OpenVMS Alpha service in the vector, not SS$_ILLSER"
status = "todo"
note = "Stubs now: $DLCEFC, $CNTREG, $SETPRT, $LKWSET, $ULWSET, $LCKPAG, $ULKPAG; most others are missing"

[[feature]]
id = "executive-mode"
title = "RMS and the other executive mode services run in executive mode, not kernel"
status = "todo"
note = "PRD-0003 tier 3 #23: with the mode threads of ADR-0005"

[[feature]]
id = "services-64"
title = "The _64 services and 64-bit item lists and descriptors"
status = "todo"
note = "ADR-0023 follow-up: the 64-bit address space, its own ADR"

[[feature]]
id = "user-written"
title = "User-written system services: privileged shareable images with a change mode vector"
status = "todo"
note = "Needs t8y:/exec/installed-images, a PRD-0003 non-goal"

[[feature]]
id = "sslog"
title = "System service logging: SET PROCESS/SSLOG and ANALYZE/SSLOG"
status = "todo"

[[needs]]
on = "/platform/pal"
why = "Delivers CHMK and CHME through the SCB, and REI back"

[[needs]]
on = "/exec/privileges"
why = "CMKRNL and CMEXEC"

[[needs]]
on = "/exec/memory"
why = "The vector's pages are made user readable at start"
soft = true

[[needs]]
on = "/exec/installed-images"
why = "User-written services live in privileged shareable images, installed /PROTECTED"
soft = true
+++

A VMS program calls a system service like any routine, `SYS$QIOW`
with `CALLS` or the `$QIOW_S` macro. The entry point is in the system
service vector, a page every mode can read and run, and it changes mode
with `CHMK` (kernel) or `CHME` (executive) and a code; the change mode
dispatcher in the executive checks the code and the argument count,
calls the service's `EXE$` routine, and returns to the caller's mode
with the status in R0. Each service probes the caller's arguments in
the caller's mode before using them. A few services, such as
`$UNWIND` and `$PUTMSG`, run in the caller's mode without changing
mode. On Alpha the vector moved into `SYS$PUBLIC_VECTORS.EXE`, which
programs link against. Sites add their own services as privileged
shareable images with a change mode vector, installed with `INSTALL
/SHARE /PROTECT`.

vaxpunk's are in `vms/exec/syssrv.mar`: `SYS$name` routines in the
`EXEC$VECTOR` psect, page aligned and made user readable at start, a
table of `EXE$name` addresses and argument counts, and the `CHMK` and
`CHME` dispatchers, which since PRD-0005 keep the arguments in
registers, copy those past the eighth from the caller's stack, refuse
one that isn't a sign-extended longword (`SS$_ARG_GTR_32_BITS`), and
`REI` with R0 and R1 in the PAL's frame
([DESIGN-0004](../../design/0004-calling-standard.md)). 84
services work, grouped as DESIGN-0002's table shows; nine more, the
memory services that need global sections or paging and `$DLCEFC`,
are stubs that return `SS$_ILLSER`. Programs link against `SYS.STB`, made from
`EXEC.EXE`'s map. RMS's services run in kernel mode, where VMS runs
them in executive mode.
