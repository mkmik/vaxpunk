+++
title = "Interrupt-driven I/O"
summary = "Devices that queue requests and complete them by interrupt and fork"
aka = ["interrupt service routine", "ISR", "fork process", "IOC$REQCOM", "IPL$_IOPOST", "start I/O", "asynchronous disk I/O", "seL4 IRQ handler"]
kind = "core"
intent = "goal"
research = "outline"
status = "done"
refs = ["docs/adr/0047-interrupt-driven-disk-io-and-locked-pages.md", "docs/prd/0003-multi-user-vms.md", "docs/adr/0015-file-system-io-through-the-disk-driver.md", "docs/adr/0020-file-system-lock-below-ipl-synch.md", "docs/design/0001-pal-interface.md", "pal/src/main.c", "vms/exec/qio.mar"]

[[needs]]
on = "/platform/pal"
why = "The PAL turns a device's interrupt into one at the device's IPL"

[[needs]]
on = "/platform/sel4"
why = "Device interrupts reach the PAL through seL4's IRQ handlers"

[[needs]]
on = "/platform/virtio"
why = "virtio-blk's used ring and interrupt"

[[needs]]
on = "/exec/qio"
why = "IRPs wait on the UCB; IOC$CANCEL must wait for those the device holds"

[[needs]]
on = "/exec/ipl-sync"
why = "Interrupt at device IPL, fork down to IPL$_SYNCH"
+++

On VMS a driver starts a request and returns; the device interrupts
when it is done, the interrupt service routine queues a fork process at
a lower IPL, and the fork completes the IRP and starts the next. So the
CPU runs other processes while a disk seeks, and a process waits for its
I/O in an event flag wait, not a loop. `$CANCEL` and rundown must wait
for requests the device already holds.

vaxpunk's disks work this way
([ADR-0047](../../adr/0047-interrupt-driven-disk-io-and-locked-pages.md)).
The PAL takes each disk's interrupt through seL4's IRQ handler;
`DISKSTART` hands the device the request's pages, which the driver
locked by PFN (`EXE$READLOCK`, `MMG$IOLOCK`), and returns. When the
device is done, the PAL requests an interrupt at IPL 21 through
`SCB$L_DISK`; `DK$INTERRUPT` forks to `DK$FORK` at `IPL$_SYNCH`, which
takes the status with `DISKDONE`, completes the IRP and starts the next
one queued on the UCB (`EXE$INSIOQ`, `IOC$REQCOM`). The file system
waits for its IRP while other processes run, and rundown waits for a
request the device has (`DK$CANCEL`). The TCP/IP port already worked
this way: its responses come by interrupt. The console UART is still
looked at on each 10 ms tick (t8y:/drivers/console#interrupt-uart).
