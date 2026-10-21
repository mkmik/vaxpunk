+++
title = "Startup and shutdown"
summary = "STARTUP.COM and the site's SYSTARTUP_VMS.COM at boot; SHUTDOWN.COM and OPCCRASH"
aka = ["STARTUP", "STARTUP.COM", "SYSTARTUP_VMS.COM", "SYLOGICALS.COM", "SYCONFIG.COM", "SYSECURITY.COM", "SYPAGSWPFILES.COM", "SHUTDOWN.COM", "SYSHUTDWN.COM", "OPCCRASH", "STARTUP_P1", "VMS$PHASES.DAT", "STARTUP$STARTUP_LAYERED", "SYLOGIN.COM"]
kind = "core"
intent = "goal"
research = "done"
refs = ["docs/boot.md", "docs/adr/0029-loginout-and-sysuaf.md", "docs/design/0002-executive-processes.md", "vms/sysmgr/systartup_vms.com", "vms/sysmgr/sylogin.com", "vms/sysexe/shutdown.com", "vms/sysexe/opccrash.mar", "vms/exec/exec.mar", "docs/adr/0008-host-powers-qemu-off-on-halt.md"]

[[feature]]
id = "systartup"
title = "SYSTARTUP_VMS.COM at boot: the ramdisk, SYS$SYSROOT, OPCOM, TCP/IP, the data disk"
status = "done"
milestone = "m0"
note = "boot.md section 6"

[[feature]]
id = "startup-process"
title = "A STARTUP process runs it before anyone logs in, and ends with job terminated"
status = "done"
milestone = "m1"
note = "PRD-0003 step 4, ADR-0029; DESIGN-0002"

[[feature]]
id = "startup-params"
title = "P1 to P8 from the SYSGEN parameters STARTUP_P1 to STARTUP_P8"
status = "done"
note = "PR #140; F$GETSYI"
needs = ["/exec/sysgen-parameters"]

[[feature]]
id = "sylogin"
title = "SYLOGIN.COM at every login"
status = "done"
milestone = "m1"
note = "ADR-0029 decision 5"

[[feature]]
id = "shutdown"
title = "SHUTDOWN.COM ends with MCR OPCCRASH, which halts with CMKRNL"
status = "done"
note = "PR #137; ADR-0029 decision 7; power off under TCG only (PRD-0003 tier 4 #42)"

[[feature]]
id = "orderly-shutdown"
title = "SHUTDOWN asks minutes and reason, warns users, stops queues and processes, dismounts"
status = "todo"
needs = ["/utils/reply#broadcast", "/utils/job-controller#queue-commands"]
note = "shutdown.com ponytail"

[[feature]]
id = "reboot"
title = "Automatic reboot and the shutdown options (REBOOT_CHECK, POWER_OFF, ...)"
status = "todo"

[[feature]]
id = "persistent-specific"
title = "SYS$SPECIFIC as a directory root that survives a reboot, not the ramdisk"
status = "todo"
note = "PRD-0003 tier 3 #25: rooted logical names"

[[feature]]
id = "startup-phases"
title = "STARTUP.COM's phases and the startup databases, run from SYSMAN STARTUP"
status = "todo"
note = "SYSMAN STARTUP edits the databases (t8y:/utils/sysman)"

[[feature]]
id = "site-procedures"
title = "SYLOGICALS, SYCONFIG, SYSECURITY, SYPAGSWPFILES and SYSHUTDWN as hooks"
status = "todo"

[[feature]]
id = "minimum-boot"
title = "STARTUP_P1 MIN: a minimum startup, as VMS's STARTUP.COM does it"
status = "todo"

[[feature]]
id = "sys-node"
title = "SYS$NODE, NODE::, in the system table from SCSNODE"
status = "todo"
needs = ["/exec/sysgen-parameters#scsnode"]

[[needs]]
on = "/exec/system-init"
why = "EXEC$START creates the STARTUP process at the end of system initialization"

[[needs]]
on = "/dcl/procedures"
why = "STARTUP's DCL runs SYSTARTUP_VMS.COM as a command procedure"

[[needs]]
on = "/dcl/lexicals"
why = "F$GETSYI for STARTUP_P1 to P8"

[[needs]]
on = "/utils/mount"
why = "Makes and mounts the ramdisk and the data disk"

[[needs]]
on = "/net/tcpip-utility"
why = "TCPIP START COMMUNICATION brings the network up"

[[needs]]
on = "/exec/logical-names"
why = "Defines SYS$SYSROOT, SYS$SYSTEM and the other system logical names"

[[needs]]
on = "/exec/privileges"
why = "OPCCRASH needs CMKRNL"
+++

On VMS the system starts in a `STARTUP` process that runs
`SYS$SYSTEM:STARTUP.COM`. Since V5 that procedure walks phases
(`INITIAL`, `DEVICES`, `CONFIG`, `BASEENVIRON`, `LPBEGIN`, `LPMAIN`,
`LPBETA`, `END`) listed in startup databases, which `SYSMAN STARTUP`
edits, and calls the site's hooks on the way: `SYLOGICALS.COM`,
`SYCONFIG.COM`, `SYPAGSWPFILES.COM`, `SYSECURITY.COM`, and last
`SYS$MANAGER:SYSTARTUP_VMS.COM`, where a site starts its queues,
network and layered products. With `STARTUP_P1` set to `MIN` it starts
only the base system. `SYLOGIN.COM` runs at every login. To stop,
`@SYS$SYSTEM:SHUTDOWN` asks how many minutes and why, warns logged-in
users, runs `SYSHUTDWN.COM`, stops queues and user processes, dismounts
volumes and runs `OPCCRASH`, which halts or reboots. `OPCCRASH` alone is
the emergency stop.

vaxpunk has no `STARTUP.COM`: the executive's `EXEC$START` creates a
`STARTUP` process whose DCL runs `SYSTARTUP_VMS.COM` directly
([boot.md](../../boot.md),
[ADR-0029](../../adr/0029-loginout-and-sysuaf.md)). It sets `P1` to
`P8` from `STARTUP_P1-8`, makes and mounts the ramdisk as `SYS$SPECIFIC`,
defines `SYS$SYSROOT` and the system logical names, copies `SYSUAF.DAT`,
starts TCP/IP and TELNETD, mounts the data disk, and logs out. Then the
swapper starts `LOGINOUT` on the console. `SHUTDOWN.COM` prints VMS's
banner and runs `MCR OPCCRASH`, which halts in kernel mode; the
ponytail says there are no questions, no warnings and no stopping of
other processes first.
