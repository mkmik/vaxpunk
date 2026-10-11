+++
title = "Drivers"
band = "exec"
summary = "Device drivers the executive's $QIO calls"
+++

A VMS device driver is a loadable image in the executive: FDT routines
that check each `$QIO` function, a start I/O routine that works the
device's queue of IRPs, and an interrupt service routine and fork
processing that complete them. The I/O database (DDBs, UCBs, CRBs, IDBs)
says which units exist; `SYSMAN IO AUTOCONFIGURE` finds the hardware and
loads the drivers. The I/O User's Reference Manual has a chapter for each
class: disks, magnetic tapes, mailboxes, terminals, pseudoterminals,
shadow sets, generic SCSI and LAN devices; line printers, the null
device and the console are the others a system manager meets.

vaxpunk's drivers are MACRO-32 in `vms/exec/`, linked into `EXEC.EXE`,
each a UCB and an FDT routine ([ADR-0013](../../docs/adr/0013-qio-irps-and-drivers.md)):
the terminals' (`ttdriver.mar`), the disks' (`DK$FDT` in `f11.mar`), the
ramdisk (`mddriver.mar`), the network's port driver (`netdriver.mar`)
and the mailboxes' (`mbdriver.mar`, under //exec/mailboxes). The
hardware is the PAL's: it drives virtio and the UART, polled, and hands
the executive blocks, characters and port messages. Mailboxes belong to
//exec/mailboxes, not here.
