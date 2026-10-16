+++
title = "SRM console"
summary = "The >>> prompt before VMS: boots it, holds the boot settings in NVRAM, catches it when it halts"
aka = ["SRM", ">>>", "console firmware", "HWRPB", "hardware restart parameter block", "NVRAM", "auto_action", "bootdef_dev", "boot_osflags", "boot_file", "show device", "examine", "deposit", "nvram script", "Ctrl/P", "callbacks", "CRB", "secure mode"]
kind = "platform"
intent = "goal"
research = "outline"
refs = ["docs/prd/0010-srm-console.md", "docs/boot.md"]

[[feature]]
id = "prompt"
title = "Banner and >>>: line editing and recall, keyword prefixes, help, show memory, config, version, pal, init"
status = "todo"
note = "PRD-0010 phase 1"

[[feature]]
id = "envars"
title = "show, set and clear of environment variables, kept across power cycles in emulated NVRAM"
status = "todo"
note = "PRD-0010 phase 1: a host file first, a flash device later; auto_action, bootdef_dev, boot_osflags, boot_file, boot_reset"

[[feature]]
id = "devices"
title = "show device: SRM's long and short names (dka0.0.0.7.0, DKA0) for the virtio disks, ewa0, dqa0"
status = "todo"
needs = ["/platform/virtio"]
note = "PRD-0010 phase 1; stable for one QEMU command line"

[[feature]]
id = "auto-boot"
title = "boot [dev] and auto_action boot with a countdown Ctrl/C interrupts"
status = "todo"
needs = ["#envars", "#devices"]
note = "PRD-0010 phase 1, done when QEMU auto-boots VMS from bootdef_dev and Ctrl/C leaves >>>"

[[feature]]
id = "hwrpb"
title = "Boot block, primary bootstrap and the HWRPB: memory descriptors, the per-CPU slot, the boot parameters, ARM64's fields"
status = "todo"
needs = ["#auto-boot"]
note = "PRD-0010 phase 2; takes over what t8y:/platform/pal#boot-exec does today"

[[feature]]
id = "flags"
title = "boot -flags root,flags and boot_osflags reach VMS: system root and boot flags"
status = "todo"
needs = ["#hwrpb"]
note = "PRD-0010 phase 2; conversational boot is t8y:/utils/sysgen#conversational-boot"

[[feature]]
id = "halt-continue"
title = "Back to >>> on Ctrl/P, a VMS halt or a fatal error; boot -halt; continue"
status = "todo"
needs = ["#hwrpb"]
note = "PRD-0010 phase 2; replaces powering QEMU off on a halt (ADR-0008)"

[[feature]]
id = "examine-deposit"
title = "examine and deposit of memory (pmem:, vmem:) and registers"
status = "todo"
needs = ["#halt-continue"]
note = "PRD-0010 phase 2"

[[feature]]
id = "restart"
title = "crash, and auto_action restart: VMS's restart routine writes the dump, then the machine reboots"
status = "todo"
needs = ["#halt-continue", "/exec/crash-dump#dump-file"]
note = "PRD-0010 phase 2"

[[feature]]
id = "nvram-script"
title = "The nvram power-up script, RAM files, edit, more, cat, echo, sleep; starting sidecars before VMS"
status = "todo"
needs = ["#envars"]
note = "PRD-0010 phase 2; which helper starts a sidecar (lwIP today) is an open question"

[[feature]]
id = "callbacks"
title = "Console callbacks through the HWRPB: getc, puts, block I/O, getenv, setenv, save_env, reset_term"
status = "todo"
needs = ["#hwrpb"]
note = "PRD-0010 phase 2; for the bootstrap, SYSBOOT and the crash dump before VMS has drivers"

[[feature]]
id = "secure"
title = "set password, set secure, login: secure mode"
status = "todo"
needs = ["#envars"]
note = "PRD-0010 phase 3; DS10 reference 2.8, 2.9"

[[feature]]
id = "jobs"
title = "Background jobs with &, ps, kill, show_status, kill_diags; pipes and redirection"
status = "todo"
note = "PRD-0010 phase 3; ps, kill, ls not yet confirmed in a manual"

[[feature]]
id = "diagnostics"
title = "test and memtest"
status = "todo"
needs = ["#jobs"]
note = "PRD-0010 phase 3"

[[feature]]
id = "event-log"
title = "An event log in NVRAM, read with more el"
status = "todo"
needs = ["#nvram-script"]
note = "PRD-0010 phase 3; contents and size open"

[[feature]]
id = "halt-assertion"
title = "Halt assertion: a host flag or an early key skips the nvram script and auto-boot once"
status = "todo"
needs = ["#nvram-script"]
note = "PRD-0010 phase 3"

[[feature]]
id = "owns-pal"
title = "The PAL moves into the console; show pal reports vaxpunk's own"
status = "todo"
needs = ["#hwrpb"]
note = "PRD-0010 phase 4; planned in a separate document"

[[needs]]
on = "/platform/console-uart"
why = "The console is serial only"

[[needs]]
on = "/platform/virtio#blk"
why = "Reads the boot block and the primary bootstrap from the boot disk"

[[needs]]
fuzzy = "How the console is built on the current substrate: language, where it runs relative to seL4 and the PAL"
why = "PRD-0010 leaves it to a separate document"

[[needs]]
fuzzy = "Emulated NVRAM: a host file presented to the guest as a flash device"
why = "Variables, the password, the nvram script and the event log outlive a power cycle"
+++

On an Alpha, the SRM console was the firmware under OpenVMS: the `>>>`
prompt the machine sat at before VMS ran and went back to when VMS halted
or crashed. It showed the boot devices, kept boot settings in NVRAM
(`bootdef_dev`, `boot_osflags`, `auto_action`), booted VMS with chosen
flags and system root, examined a stopped machine, and supplied the
PALcode. The HWRPB was its contract with the OS.

vaxpunk has no console: QEMU boots straight through the boot chain into
the PAL, which loads `EXEC.EXE`, and a halt powers QEMU off
([ADR-0008](../../adr/0008-host-powers-qemu-off-on-halt.md)).
[PRD-0010](../../prd/0010-srm-console.md) describes the console vaxpunk
wants, with SRM's commands, variables, device names and boot flow, in
four phases: a prompt that boots, a real handoff and a way back, operator
comforts, and the console owning the machine.
