+++
title = "boot runner and QEMU tests"
summary = "cargo run -p boot: makes the ESP, boots QEMU, drives the console in tests"
aka = ["boot crate", "cargo run -p boot", "just boot", "run-qemu.sh", "mkesp.sh", "esp.img", "serial.log"]
kind = "tool"
intent = "goal"
research = "done"
milestone = "m0"
refs = ["boot/README.md", "boot/main.rs", "boot/mkesp.sh", "boot/tests", "scripts/run-qemu.sh", "docs/adr/0008-host-powers-qemu-off-on-halt.md"]

[[feature]]
id = "boot"
title = "ESP with Limine, the shim, seL4 and the root task; QEMU with the system disk"
status = "done"
milestone = "m0"

[[feature]]
id = "tests"
title = "Boot tests driving DCL on the console: STARTUP, a session, TCP/IP, RMS files"
status = "done"
milestone = "m0"
note = "boot/tests/boot.rs, network.rs, rms.rs; CI runs them under TCG"

[[feature]]
id = "options"
title = "--gdb, --hvf, a second UART on telnet, STARTUP_P1 to P8, --autologin"
status = "done"
note = "PRs #139, #140"

[[feature]]
id = "two-systems"
title = "Two vaxpunks on one network, each with its own disk and MAC"
status = "done"
note = "run-qemu.sh: DATADISK, NETDEV, MAC"

[[needs]]
on = "/host/sysdisk-builder"
why = "Boots the disk it writes"

[[needs]]
on = "/platform/boot"
why = "Puts EDK2, Limine and the shim together"
soft = true

[[needs]]
on = "/host/ods"
why = "Writes SYSALF.DAT for --autologin and checks the data disk after the tests"

+++

On real VMS you boot from the console with `BOOT`, and test by hand or
with DEC's test packages. vaxpunk's `boot` crate does it on the host:
`cargo run -p boot` copies the kernel, shim, root task and system disk to
`out/`, makes the FAT32 ESP with Limine and mtools, and becomes
`scripts/run-qemu.sh`, which boots EDK2, Limine, the shim, seL4 and the
PAL in QEMU's `virt` machine. `just boot` logs the console in as
`SYSTEM`.

Its tests boot the whole system without a terminal and type at DCL's
prompt, a phase at a time with deadlines, and compare what it prints;
the RMS test compares with what real OpenVMS printed for the same files.
The console log is `out/serial.log`.
