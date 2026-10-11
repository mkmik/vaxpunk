+++
title = "Real hardware (Apple Silicon, Raspberry Pi)"
summary = "Booting on real ARM64 machines instead of QEMU's virt board"
aka = ["bare metal", "Apple Silicon", "Raspberry Pi", "Asahi", "real boards"]
kind = "platform"
intent = "aspiration"
research = "deferred"
status = "todo"

[[needs]]
on = "/platform/pal#dtb-devices"
why = "Device addresses from the board's DTB"

[[needs]]
on = "/platform/pal#device-interrupts"
why = "Real devices interrupt; polling QEMU's fixed addresses won't do"

[[needs]]
on = "/platform/sel4"
why = "A seL4 build for the board, or the native kernel"

[[needs]]
fuzzy = "Drivers for the board's storage, network and UART instead of virtio"
why = "No virtio on real hardware"

[[needs]]
fuzzy = "A timer device driven by the PAL instead of the clock thread"
why = "ADR-0004: real boards have a free timer"

[[needs]]
fuzzy = "Each board's boot path: UEFI on a Pi, m1n1 on Apple Silicon"
why = "Limine and EDK2 are QEMU's today"
+++

VMS ran on DEC's own machines, and later on HP's Integrity servers and,
with VSI, on x86-64 servers and hypervisors. Each port needed drivers for
the machine's buses, disks, network controllers and console, and the
machine check and error logging code for its hardware.

vaxpunk runs only on QEMU's `virt` machine, under TCG or HVF, and in the
browser demo. Real ARM64 hardware, such as Apple Silicon Macs or a
Raspberry Pi, is an aspiration with nothing designed: it needs device
addresses from the DTB and device interrupts in the PAL (PRD-0003 tier 4
#38), drivers for real devices instead of virtio, a timer device instead
of the PAL's clock thread
([ADR-0004](../../adr/0004-interval-timer-is-a-pal-thread.md)), and
each board's boot path.
