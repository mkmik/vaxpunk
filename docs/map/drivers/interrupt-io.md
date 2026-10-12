+++
title = "Interrupt-driven I/O"
summary = "Devices that queue requests and complete them by interrupt and fork"
aka = ["interrupt service routine", "ISR", "fork process", "IOC$REQCOM", "IPL$_IOPOST", "start I/O", "asynchronous disk I/O", "seL4 IRQ handler"]
kind = "core"
intent = "goal"
research = "outline"
status = "todo"
refs = ["docs/prd/0003-multi-user-vms.md", "docs/adr/0015-file-system-io-through-the-disk-driver.md", "docs/adr/0020-file-system-lock-below-ipl-synch.md", "docs/design/0001-pal-interface.md", "pal/src/main.c", "vms/exec/qio.mar"]

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

vaxpunk's disks are polled: the PAL starts each virtio-blk request and
waits for it, so `DK$STARTIO` completes the IRP before it returns
(`pal/src/main.c`, [ADR-0015](../../adr/0015-file-system-io-through-the-disk-driver.md)).
The console UART is looked at on each 10 ms tick. PRD-0003 tier 4 #37
and #38 keep it so until the hardware or the load changes; then the
PAL takes the devices' interrupts through seL4, the disk driver queues,
the file system waits for its IRP (t8y:/files/ods2), and
`IOC$CANCEL` waits for rundown's cancels (tier 2 #18). The TCP/IP port
already works this way: responses come by interrupt.
