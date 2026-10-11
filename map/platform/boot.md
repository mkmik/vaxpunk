+++
title = "Boot chain (EDK2, Limine, shim)"
summary = "UEFI firmware, Limine and a shim that loads seL4 and the root task"
aka = ["EDK2", "UEFI", "Limine", "BOOTAA64.EFI", "shim", "shim.elf", "ESP", "esp.img", "limine.conf", "APB", "VMB", "primary bootstrap"]
kind = "platform"
intent = "goal"
research = "done"
milestone = "m0"
refs = ["boot/README.md", "pal/shim/README.md", "docs/boot.md", "boot/limine.conf", "boot/mkesp.sh", "pal/shim/src/main.c", "scripts/run-qemu.sh", "docs/adr/0008-host-powers-qemu-off-on-halt.md"]

[[feature]]
id = "uefi-limine"
title = "EDK2 boots the ESP; Limine loads the shim with seL4 and the root task"
status = "done"
milestone = "m0"
note = "boot/mkesp.sh makes the FAT32 ESP with mtools; boot/limine.conf"

[[feature]]
id = "shim"
title = "The shim places seL4, the DTB and the root task and enters seL4 as the elfloader does"
status = "done"
milestone = "m0"
note = "pal/shim/README.md: the entry contract read from seL4 16.0.0's source"

[[feature]]
id = "startup-params"
title = "STARTUP_P1 to STARTUP_P8 from cargo run -p boot -- --p1=..., through fw_cfg"
status = "done"
note = "PR #140; the PAL copies them into the RPB"

[[feature]]
id = "power-off"
title = "A halt powers QEMU off: %PAL-I-POWEROFF, and serial-filter.py ends QEMU"
status = "done"
note = "ADR-0008; only when QEMU runs through run-qemu.sh"

[[feature]]
id = "boot-flags"
title = "Boot flags, a system root and SYSBOOT> before the executive starts"
status = "todo"
note = "VMS's R5 boot flags and conversational boot; only STARTUP_P1-8 pass today"

[[needs]]
on = "//platform/sel4"
why = "The shim loads the kernel and enters it"

[[needs]]
on = "//platform/pal"
why = "The root task is Limine's second module"

[[needs]]
on = "//host/boot-runner"
why = "cargo run -p boot builds the ESP and runs run-qemu.sh"
soft = true

[[needs]]
on = "//host/sysdisk-builder"
why = "sysdisk.img, the ODS-2 volume the PAL reads EXEC.EXE from"
+++

An Alpha booted VMS from the SRM console: `BOOT DKA0` loaded the primary
bootstrap, `APB.EXE`, which read the executive from the system disk,
built the restart parameter block and started it. The boot flags in R5
chose things such as conversational boot, where `SYSBOOT>` lets the
operator change system parameters before the executive starts, and the
system root, `[SYSn]`. A VAX did the same with `VMB.EXE`.

vaxpunk boots on QEMU's `virt` machine:
`EDK2 -> Limine -> shim -> seL4 -> root task -> EXEC.EXE`
([docs/boot.md](../../docs/boot.md)). `boot/mkesp.sh` puts Limine,
`limine.conf`, the shim, `kernel.elf` and `roottask.elf` on a FAT32 ESP.
The shim ([pal/shim/README.md](../../pal/shim/README.md)) finds the
PL011 in the DTB to print, copies the kernel, DTB and root task into
place and enters seL4 in the state seL4's own elfloader leaves it. The
part APB played, reading `[SYSEXE]EXEC.EXE` from Files-11 and building
the RPB, is the PAL's. A halt prints `%PAL-I-POWEROFF` and the host's
console filter stops QEMU
([ADR-0008](../../docs/adr/0008-host-powers-qemu-off-on-halt.md)). The
web demo boots the same ESP in QEMU compiled to WebAssembly. There are
no boot flags or system roots: `--p1` to `--p8` are the only parameters.
