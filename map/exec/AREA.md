+++
title = "Executive"
band = "exec"
summary = "The VMS kernel: processes, memory, synchronization, I/O, security"
+++

The executive is the part of VMS that runs in kernel and executive
mode: processes and the scheduler, virtual memory, paging and
swapping, pool, IPL, ASTs and event flags, timers, logical names,
`$QIO` and the I/O database, mailboxes, the lock manager, condition
handling, the image activator, the system service vector and the
calling standard, and the security model (UICs and protection,
privileges, identifiers, ACLs, auditing, intrusion detection). It also
holds the quotas and accounting each process is charged, the
information services that report on all of it, messages, the system
parameters, bugchecks, error logging, and the code that starts the
system. Clusters, DECdtm and Galaxy reach across nodes and instances
from here. Drivers are in `map/drivers`; the file system's XQP, though
it runs in the executive, is in `map/files`.

vaxpunk's executive is `vms/exec/`: MACRO-32, with `$NUMTIM` in
BLISS-64, linked into one image, `EXEC.EXE`, that the PAL loads and
calls through VAX privileged instructions
([DESIGN-0002](../../docs/design/0002-executive-processes.md),
[ADR-0002](../../docs/adr/0002-root-task-is-the-pal.md)). It runs on
one CPU, synchronizes with IPL, keeps every page it allocates, and
follows the vaxpunk calling standard
([ADR-0023](../../docs/adr/0023-calling-standard.md)). Milestone 1,
[PRD-0003](../../docs/prd/0003-multi-user-vms.md), adds what several
users need: privileges, UICs and file protection (done), quotas and
accounting, and a lock manager. Paging, swapping, installed images,
ACLs, auditing, clusters and SMP are its non-goals, and stay here as
aspirations.
