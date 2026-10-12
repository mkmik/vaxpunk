+++
title = "Native kernel without seL4"
summary = "The PAL and everything below it in MACRO-32 and BLISS, on bare ARM64 at EL1"
aka = ["native PAL", "no seL4", "self-hosted kernel", "SWIS"]
kind = "platform"
intent = "aspiration"
research = "deferred"
status = "todo"
refs = ["README.md", "docs/adr/0001-pal-interface-vms-vocabulary.md", "docs/prd/0002-networking.md"]

[[needs]]
on = "/platform/pal"
why = "Keeps its interface, so the executive doesn't change"

[[needs]]
on = "/lang/macro32"
why = "The main kernel language"

[[needs]]
on = "/lang/bliss"
why = "Longer logic, as in VMS"

[[needs]]
fuzzy = "Real page tables, exception vectors, the GIC and the generic timer at EL1"
why = "What seL4 does today"

[[needs]]
fuzzy = "A way to emulate four access modes on EL0 and EL1 without seL4's threads"
why = "ADR-0005's design depends on seL4 address spaces"

[[needs]]
fuzzy = "Native drivers for the disks, network and console, and a TCP/IP stack in place of lwIP"
why = "Everything C below the port and in the PAL goes"
+++

Marko's stated long-term goal is a vaxpunk that is fully native and
self-hosted, everything in MACRO-32 and BLISS, with seL4 gone. seL4 (MCS)
and the C PAL are bootstrap scaffolding. A native kernel would run at
EL1 on bare ARM64, write real page tables, take exceptions and device
interrupts itself, and serve the same PAL calls, as Itanium's SWIS did
for OpenVMS I64 where there was no PALcode.

The PAL's interface was made for this: it speaks only VMS vocabulary
([ADR-0001](../../adr/0001-pal-interface-vms-vocabulary.md)), so
the executive need not change when what is under it does. The TCP/IP
port was drawn so a native stack can replace lwIP the same way
([PRD-0002](../../prd/0002-networking.md)). Not designed yet: how
four modes map onto EL0 and EL1, what replaces the C drivers, and how
the Linux-binary escape hatch survives seL4's removal.
