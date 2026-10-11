+++
title = "SYSGEN"
summary = "Shows and changes the system parameters; AUTOGEN computes them"
aka = ["SYSGEN", "SYSGEN.EXE", "SYSGEN>", "SYSBOOT", "SYSBOOT>", "AUTOGEN", "MODPARAMS.DAT", "ALPHAVMSSYS.PAR", "conversational boot", "STARTUP_P1"]
kind = "core"
intent = "aspiration"
research = "outline"
refs = ["docs/design/0001-pal-interface.md", "docs/boot.md", "vms/sysmgr/systartup_vms.com", "https://docs.vmssoftware.com/vsi-openvms-system-management-utilities-reference-manual-volume-ii-m-z/", "https://docs.vmssoftware.com/vsi-openvms-system-management-utilities-reference-manual-volume-i-a-l/"]

[[feature]]
id = "parameters"
title = "SHOW, SET, USE CURRENT and WRITE CURRENT: the parameter file"
status = "todo"

[[feature]]
id = "active"
title = "WRITE ACTIVE: dynamic parameters changed on the running system"
status = "todo"
needs = [":parameters"]

[[feature]]
id = "conversational-boot"
title = "Conversational boot: SYSBOOT> changes parameters before the system starts"
status = "todo"
needs = ["//platform/boot"]
note = "vaxpunk's cargo run -p boot -- --p1 to --p8 set STARTUP_P1 to P8 only (PR #140, DESIGN-0001 RPB$T_STARTUP)"

[[feature]]
id = "page-swap-files"
title = "CREATE and INSTALL page, swap and dump files"
status = "todo"
needs = ["//exec/paging", "//exec/swapping", "//exec/crash-dump"]
note = "Paging and swapping are non-goals of Milestone 1"

[[feature]]
id = "autogen"
title = "AUTOGEN: parameters and file sizes from the hardware, the load and MODPARAMS.DAT"
status = "todo"
needs = [":parameters", "//dcl/procedures"]

[[needs]]
on = "//exec/sysgen-parameters"
why = "The parameters it shows and changes, in the executive and the parameter file"

[[needs]]
on = "//exec/system-init"
why = "The parameter file is read at boot"

[[needs]]
on = "//exec/privileges"
why = "CMKRNL to change active parameters"

[[needs]]
on = "//rms/core"
why = "Reads and writes the parameter file"

[[needs]]
on = "//rtl/cli"
why = "Its own commands at the SYSGEN> prompt"
+++

VMS sizes its tables and pools, and sets its limits and behaviour, from
several hundred system parameters: memory and pool sizes, working set
and quota minimums (`PQL_`), the lock manager's tables, scheduling
quanta, security and login settings, and the eight `STARTUP_P1` to
`STARTUP_P8` that tell the startup procedure how to start. `SYSGEN`
shows and changes them, in the parameter file the next boot reads
(`ALPHAVMSSYS.PAR` on Alpha) or, for the dynamic ones, in the running
system. A conversational boot stops at `SYSBOOT>` so they can be
changed before the executive starts, which is how a broken system is
brought up with `STARTUP_P1 "MIN"`. `AUTOGEN`, a command procedure,
computes them from the hardware, feedback from the running system and
the site's `MODPARAMS.DAT`, and sizes the page, swap and dump files that
SYSGEN creates. On Alpha, SYSMAN's `PARAMETERS` and `IO` commands took
over part of SYSGEN's work.

vaxpunk has no `SYSGEN`. The executive's sizes are constants in its
sources. The only parameters are `STARTUP_P1` to `STARTUP_P8`, which
`cargo run -p boot -- --p1=MIN` hands to the PAL through QEMU's firmware
configuration device, and `SYSTARTUP_VMS.COM` reads with `F$GETSYI`.
