+++
title = "System parameters"
summary = "The SYSGEN parameters: sizes, limits and switches the executive reads at boot"
aka = ["SYSGEN parameters", "SGN$", "ALPHAVMSSYS.PAR", "X86_64VMSSYS.PAR", "VAXVMSSYS.PAR", "SYSBOOT", "conversational boot", "AUTOGEN", "MODPARAMS.DAT", "dynamic parameters", "SYSMAN PARAMETERS", "STARTUP_P1", "QUANTUM", "MAXPROCESSCNT", "NPAGEDYN", "MAXSYSGROUP", "F$GETSYI"]
kind = "core"
intent = "goal"
research = "outline"
refs = ["docs/boot.md", "vms/exec/exec.mar", "vms/exec/sched.mar", "vms/exec/getjpi.mar", "vms/exec/f11.mar", "vms/exec/mbdriver.mar", "vms/sysmgr/systartup_vms.com", "pal/src/main.c", "scripts/run-qemu.sh"]

[[feature]]
id = "startup-p"
title = "STARTUP_P1 to STARTUP_P8 from the host, through the RPB, for $GETSYI and SYSTARTUP_VMS.COM"
status = "done"
note = "Commit 6e5df3f; cargo run -p boot -- --p1=VALUE, QEMU's fw_cfg, SGN$GB_STARTUP_P1"

[[feature]]
id = "quantum"
title = "QUANTUM in SGN$GW_QUANTUM, which $GETSYI returns"
status = "done"
milestone = "m0"
note = "sched.mar; fixed at build time"

[[feature]]
id = "constants"
title = "Parameters kept as constants: MAXSYSGROUP, DEFMBXMXMSG, DEFMBXBUFQUO"
status = "done"
note = "f11.mar, mbdriver.mar; named as VMS's SYSGEN names them; MAXPROCESSCNT, NPAGEDYN and NPAGEVIR are cells now (SGN$GW_MAXPRCCT, SGN$GL_NPAGEDYN, SGN$GL_NPAGEVIR)"

[[feature]]
id = "cells"
title = "The parameters as SGN$ cells the executive reads, not constants"
status = "todo"

[[feature]]
id = "parameter-file"
title = "A parameter file on the system disk that boot reads"
status = "todo"
needs = ["#cells"]
note = "ALPHAVMSSYS.PAR on Alpha"

[[feature]]
id = "getsyi"
title = "$GETSYI and F$GETSYI return any parameter"
status = "todo"
needs = ["#cells"]

[[feature]]
id = "conversational"
title = "Conversational boot: SYSBOOT> to show and change parameters before the executive starts"
status = "todo"
needs = ["#parameter-file"]

[[feature]]
id = "dynamic"
title = "Dynamic parameters changed on a running system: SYSGEN WRITE ACTIVE, SYSMAN PARAMETERS"
status = "todo"
needs = ["#cells"]
note = "t8y:/utils/sysgen, t8y:/utils/sysman"

[[feature]]
id = "autogen"
title = "AUTOGEN: parameters computed from the machine, MODPARAMS.DAT and feedback"
status = "todo"
needs = ["#parameter-file"]

[[feature]]
id = "scsnode"
title = "SCSNODE, the node name: up to 6 letters and digits, a default such as VAXPNK, cargo run -p boot -- --node=NAME; $GETSYI's NODENAME and SCSNODE"
status = "todo"
note = "From the host through the RPB, as STARTUP_Pn; then from #parameter-file, changed with MODPARAMS.DAT and AUTOGEN or SYSGEN. 6 characters as VMS's, from DECnet Phase IV; a longer Internet name is TCPIP$INET_HOST's"

[[needs]]
on = "/platform/pal"
why = "Passes the startup parameters in the RPB"

[[needs]]
on = "/exec/information-services"
why = "$GETSYI returns them"

[[needs]]
on = "/utils/sysgen"
why = "SYSGEN shows and sets them"
soft = true
+++

The system parameters, several hundred of them, size and tune the
executive: how much pool (`NPAGEDYN`), how many processes
(`MAXPROCESSCNT`), the scheduler's `QUANTUM`, the default working set
and mailbox sizes, the login security limits (`LGI_*`), cluster votes,
and the startup procedure's `STARTUP_P1` to `STARTUP_P8`. They live in
a parameter file on the system disk, `ALPHAVMSSYS.PAR` on Alpha, which
the bootstrap reads; at a conversational boot `SYSBOOT>` shows and
changes them before the executive starts. Dynamic ones can be changed
on a running system with `SYSGEN` or `SYSMAN`. `AUTOGEN` computes them
from the machine's memory and devices, the site's `MODPARAMS.DAT` and
feedback from the running system, and writes the file. `$GETSYI` and
`F$GETSYI` read them.

vaxpunk has a few. `STARTUP_P1` to `STARTUP_P8` come from the host:
`cargo run -p boot -- --p1=MIN` hands them to QEMU's firmware
configuration device, the PAL puts them in the RPB, `EXEC$START` copies
them to `SGN$GB_STARTUP_P1`, and `SYSTARTUP_VMS.COM` reads them with
`F$GETSYI` (docs/boot.md). `QUANTUM` is `SGN$GW_QUANTUM`. Others are
constants named after VMS's parameters: `MAXSYSGROUP` in
`f11.mar`, the default mailbox sizes in `mbdriver.mar`. There is no
parameter file, `SYSBOOT` or `SYSGEN`.
