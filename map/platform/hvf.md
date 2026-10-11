+++
title = "Hypervisor.framework (HVF) runs"
summary = "Running under macOS's Hypervisor.framework instead of TCG: best effort"
aka = ["HVF", "Hypervisor.framework", "--hvf", "-accel hvf", "TCG", "just boot"]
kind = "platform"
intent = "goal"
research = "done"
refs = ["README.md", "docs/adr/0008-host-powers-qemu-off-on-halt.md", "pal/kernel/qemu.env", "scripts/run-qemu.sh", "docs/prd/0003-multi-user-vms.md"]

[[feature]]
id = "boots"
title = "cargo run -p boot -- --hvf boots the same kernel; just boot uses it on a Mac"
status = "done"
note = "GICv3 in qemu.env so one kernel serves both; VRUN_FLAGS=--hvf for vtools's tests"

[[feature]]
id = "power-off"
title = "A halt powers QEMU off under HVF"
status = "todo"
note = "PRD-0003 tier 4 #42. ADR-0008's console-filter power-off is in run-qemu.sh and main.c, so check whether #42 is now closed"

[[feature]]
id = "cpu-model"
title = "A kernel built for -cpu host, with the 24 MHz counter"
status = "todo"
note = "README: built for Cortex-A57 at 62.5 MHz; seL4 warns under HVF. ADR-0008 calls this separate work"

[[needs]]
on = "//platform/sel4"
why = "The kernel's CPU model and timer frequency must match the host's"

[[needs]]
on = "//platform/boot"
why = "run-qemu.sh's --hvf; the halt goes through serial-filter.py"
+++

VMS has no counterpart: this is about how vaxpunk's virtual machine is
run. QEMU emulates the ARM64 CPU with TCG by default, which works on any
host, Linux CI included. On an Apple Silicon Mac, `-accel hvf` runs the
guest on the Mac's own CPU through Hypervisor.framework, much faster.

`--hvf` boots the same kernel; `pal/kernel/qemu.env` picks GICv3 because
HVF emulates no GICv2. It is best effort: the kernel is built for a
Cortex-A57, HVF offers only `-cpu host`, and the counter runs at 24 MHz,
not the 62.5 MHz seL4 was built for. PRD-0003's tier 4 #42 says power-off
works under TCG only, because semihosting's `HLT` faults under HVF.
[ADR-0008](../../docs/adr/0008-host-powers-qemu-off-on-halt.md) replaced
semihosting with a `%PAL-I-POWEROFF` line that `serial-filter.py` acts on,
and the code has it, so #42 may be stale.
