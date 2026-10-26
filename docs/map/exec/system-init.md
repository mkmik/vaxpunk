+++
title = "System initialization"
summary = "From the loaded executive to the first login: init, STARTUP, the swapper"
aka = ["EXEC$START", "SYSBOOT", "SYSINIT", "EXE$INIT", "INIT", "STARTUP process", "SYSTARTUP_VMS.COM", "STARTUP.COM", "swapper", "RPB", "loadable executive images", "execlets", "SYS$LOADABLE_IMAGES", "SYS$BASE_IMAGE", "boot flags", "system root", "SYS$SPECIFIC", "SYS$COMMON", "[SYS0.]", "MemoryDisk"]
kind = "core"
intent = "goal"
research = "done"
milestone = "m0"
refs = ["docs/boot.md", "docs/design/0002-executive-processes.md", "docs/design/0001-pal-interface.md", "docs/adr/0029-loginout-and-sysuaf.md", "vms/exec/exec.mar", "vms/sysmgr/systartup_vms.com", "vms/sysexe/shutdown.com", "pal/src/main.c"]

[[feature]]
id = "exec-start"
title = "EXEC$START: the SCB, memory, scheduler, time, console, network port, in order"
status = "done"
milestone = "m0"
note = "DESIGN-0002 Start; docs/boot.md section 5"

[[feature]]
id = "system-disk"
title = "Mount the system disk and define the system's logical names"
status = "done"
milestone = "m0"
note = "FIL$MOUNT, LNM$CREATE; %EXEC-F-NOMOUNT if it can't"

[[feature]]
id = "startup-process"
title = "A STARTUP process runs SYSTARTUP_VMS.COM before anyone logs in, and ends"
status = "done"
milestone = "m1"
note = "PRD-0003 step 4, ADR-0029; docs/boot.md section 6"

[[feature]]
id = "console-login"
title = "The swapper creates LOGINOUT on the console once STARTUP is gone, and again after each logout"
status = "done"
milestone = "m1"
needs = ["/utils/loginout"]
note = "PRD-0003 step 4, ADR-0029"

[[feature]]
id = "system-roots"
title = "SYS$SYSROOT a search list of SYS$SPECIFIC, the ramdisk, and SYS$COMMON, the system disk"
status = "done"
note = "systartup_vms.com, commit 472e9c2"

[[feature]]
id = "login-at-key"
title = "The console asks for a username at a key, not at once after a logout"
status = "done"
milestone = "m1"
note = "PRD-0003 tier 1 #2; the swapper starts it, TTY$KEY"

[[feature]]
id = "rooted-directories"
title = "Rooted system directories, [SYS0.] and [SYS0.SYSCOMMON.], not whole volumes"
status = "todo"
note = "PRD-0003 tier 3 #25; DESIGN-0002 Logical names ponytail"

[[feature]]
id = "vms-startup"
title = "VMS's own STARTUP.COM and its startup database, then the site's procedures"
status = "todo"
note = "/utils/startup-shutdown"

[[feature]]
id = "sysinit"
title = "A SYSINIT process that opens the page and swap files and mounts the system disk for the processes"
status = "todo"
note = "Needs t8y:/exec/paging, a PRD-0003 non-goal"

[[feature]]
id = "execlets"
title = "The executive as loadable images, execlets, loaded at boot from SYS$LOADABLE_IMAGES"
status = "todo"
note = "EXEC.EXE is one image, its modules linked in file-name order (vms/README.md)"

[[feature]]
id = "boot-flags"
title = "Boot flags and a minimum boot; a conversational boot"
status = "todo"
needs = ["/exec/sysgen-parameters#conversational"]

[[feature]]
id = "native-boot"
title = "Start without seL4, on the native kernel"
status = "todo"
note = "With t8y:/platform/native-kernel, a stated long-term aspiration"

[[needs]]
on = "/platform/pal"
why = "Loads EXEC.EXE from the system disk, makes the RPB, and starts it at IPL 31"

[[needs]]
on = "/files/ods2"
why = "Mounts the system disk"

[[needs]]
on = "/exec/processes"
why = "Creates STARTUP and the console's process; becomes the swapper"

[[needs]]
on = "/exec/logical-names"
why = "Defines SYS$SYSDEVICE, SYS$SYSTEM and the other system names"

[[needs]]
on = "/dcl/procedures"
why = "STARTUP's DCL runs SYSTARTUP_VMS.COM"

[[needs]]
on = "/utils/startup-shutdown"
why = "The startup and shutdown procedures it hands over to"
soft = true

[[needs]]
on = "/exec/paging"
why = "SYSINIT opens the page and swap files"
soft = true

[[needs]]
on = "/platform/native-kernel"
why = "Starting without seL4"
soft = true
+++

On VMS the console firmware loads the primary bootstrap, which reads
`SYSBOOT` from the system disk; `SYSBOOT` reads the system parameters,
loads the executive's base image and its loadable images, the
execlets, from `SYS$LOADABLE_IMAGES`, and jumps in. Each execlet's
init routine runs in turn: memory, the scheduler, the I/O database,
the drivers. The executive starts the swapper, then `SYSINIT`, a
process that opens the page and swap files and mounts the system disk
for everyone, and then `STARTUP`, a process whose DCL runs
`SYS$SYSTEM:STARTUP.COM`: the startup database of VMS and its layered
products, then the site's `SYSTARTUP_VMS.COM`, which mounts disks,
starts the network and queues, and enables logins. The system files
live in a root, `[SYS0.]` for this system and `[SYS0.SYSCOMMON.]` for
what every system booting from the disk shares, reached as
`SYS$SPECIFIC` and `SYS$COMMON` through `SYS$SYSROOT`. VSI's x86-64
boots from a memory disk the boot manager loads.

vaxpunk follows that shape on its own path ([docs/boot.md](../../boot.md)):
the PAL reads `EXEC.EXE` from the Files-11 system disk, makes the RPB
and starts it in kernel mode at IPL 31. `EXEC$START`
(`vms/exec/exec.mar`) fills the SCB, starts memory, the scheduler, the
clock, the console and the network port, mounts `DKA0:`, defines the
system logical names, creates `STARTUP` and becomes the swapper.
`STARTUP` runs `SYSTARTUP_VMS.COM`, which makes the ramdisk, makes
`SYS$SYSROOT` a search list of it and the system disk, starts TCP/IP
and `TELNETD`, and mounts the data disk; then, at a key, the swapper starts
`LOGINOUT` on the console
([ADR-0029](../../adr/0029-loginout-and-sysuaf.md)). The
executive is one image, with no execlets, `SYSINIT` or parameter file.
