# ADR-0038 — A $QIO's IRP is charged to its process's I/O quotas, and a request past them waits in $QIO and calls the FDT routine again

Oct 18, 2026 · @Marko Mikulicic

Proposed. `IOC$ALLOCIRP` charges each IRP a `$QIO` makes to its
process: a disk's request is a direct I/O, one of `DIOCNT`; any other is
a buffered one, one of `BIOCNT` and the IRP's size of `BYTCNT`, as on
VMS. `IOC$POST` gives them back, and so does whoever frees an IRP that
isn't posted. A request past what is left fails in `IOC$ALLOCIRP`; with
resource wait on, the default, `$QIO` then waits in `SCH$C_MWAIT` until
an AST comes, which a request of the process ending queues, and calls
the FDT routine again. `$SETRWM` turns resource wait off, and the request
fails with `SS$_EXQUOTA`. The limits come from the UAF at login. Step 5
of [PRD-0003](../prd/0003-multi-user-vms.md)'s work order, its first
part.

## Context

[ADR-0013](0013-qio-irps-and-drivers.md) takes an IRP from pool for each
request, with a buffered read's data in it, and caps it at 64 KB. Nothing
else limits a process: a program that queues reads on many channels takes
the pool every process needs. VMS limits each process with the UAF's
`BIOLM`, `DIOLM` and `BYTLM`.

VMS's `EXE$QIO` (VAX/VMS V4.3, `SYSQIOREQ.MAR`) charges `BIOCNT` or
`DIOCNT` itself, before it calls the FDT routines, by the function's
bit in the driver's buffered function mask. In resource wait mode it
waits there for the count (`EXE$SNGLEQUOTA`), so the FDT routine has
done nothing yet. The FDT routine charges `BYTCNT` when it allocates the
system buffer (`EXE$BUFFRQUOTA`), and waits for it there, in the middle
of the routine. `IOC$BUFPOST` and `IOC$IOPOST` give both back.

Here a driver has no function masks: one FDT routine decides each
function, and it makes the IRP with the buffer in it, so the size is
known only there. And a wait in the middle of an FDT routine is not safe
here: `SCH$WAIT` delivers the ASTs of the mode that called `$QIO`, whose
routine may deassign the channel, deleting a mailbox's UCB, or delete
the buffer's pages, under the FDT routine.

## Decision

1. **Charged with the IRP.** `IOC$ALLOCIRP` checks the quota, charges it
   and keeps in the IRP what it charged: `IRP$B_QUOTA`, `IRP$K_BIO` or
   `IRP$K_DIO`, and `IRP$L_BOFF`, the bytes. A disk's request is direct,
   any other buffered. `IOC$RETQUOTA` gives them back to the process, if
   it is still there, and clears them: `IOC$POST` first, and whoever
   frees an IRP unposted, `IOC$CANCELQ` and `NET$CANCEL` at rundown and
   an FDT routine that fails after its `IOC$ALLOCIRP`.
2. **The wait is `$QIO`'s.** A request past its quota fails in
   `IOC$ALLOCIRP` with `SS$_EXQUOTA`, which the FDT routine returns.
   With resource wait on, `IOC$ALLOCIRP` sets `PCB$V_QUOWAIT` first, and
   `$QIO` waits in `SCH$C_MWAIT` on a queue of its own until an AST
   comes, then calls the FDT routine again. The quota comes back in
   `IOC$POST`, a kernel mode AST in the process, which ends the wait. So
   an FDT routine changes nothing before its `IOC$ALLOCIRP`, which they
   all did already, as it could fail for pool.
3. **Requests done before their IRP charge nothing.** A few FDT routines
   do the request and then make an IRP only to complete it: the
   console's writes, a terminal's `IO$_SETMODE` and `IO$_SENSEMODE`,
   `IO$_TTY_PORT`, `FTA0:`'s `IO$_SETMODE`. Calling them again would do
   it twice, so they take their IRP from `IOC$ALLOCDONE`, which charges
   nothing; so does the packet of a CTRL/C or CTRL/Y AST.
4. **What can never fit fails at once**, as VMS's does: a request over
   `BYTLM`, or with a limit of 0. `$SETRWM` turns resource wait off, and
   then any shortage fails.
5. **The limits.** `EXE$LOGIN` sets them from the UAF, keeping what
   `LOGINOUT`'s own I/O still holds; `$CREPRC` gives a process its
   creator's; the swapper has `SYSTEM`'s, 150, 150 and 128,000 bytes.
   They are in the PCB, with `PCB$L_BUFIO` and `PCB$L_DIRIO`, the
   requests made. `$GETJPI` returns them, and `SHOW PROCESS/QUOTAS`
   shows what is left.

## Alternatives considered

| Option | Why not |
| --- | --- |
| Charge the counts in `$QIO` before the FDT routine, as VMS | Needs each driver's buffered and direct function masks, and the byte quota still needs charging and waiting in the FDT routine |
| Wait in `IOC$ALLOCIRP`, in the FDT routine, as VMS's `EXE$BUFFRQUOTA` | The caller's ASTs run in the wait, and may take the channel, the UCB or the buffer away from the routine |
| Wait only for the counts, and fail `BYTLM` at once | A program VMS lets wait would fail with `SS$_EXQUOTA` |
| A job information block with `BYTLM` pooled, as VMS | No subprocess shares one yet; a field moves when one does (`t8y:/exec/quotas#job-pooled`) |

## Consequences

**What gets harder.**
- A new FDT routine must make its IRP with `IOC$ALLOCIRP` before it
  changes anything, or with `IOC$ALLOCDONE` after it is done; one that
  frees an IRP it made calls `IOC$RETQUOTA` first.
- ponytail: the requests done before their IRP never wait for `BIOCNT`,
  where VMS's do; they finish at once, so no one holds pool through them.
- A process whose requests never end waits for good, as on VMS.

**What stays easy.**
- `$QIOW` behaves as before, and a process under its quotas sees no
  difference.
- The pool a process can take with I/O is bounded by its `BYTLM`.

**Follow-ups:** `ASTLM`, `TQELM`, `ENQLM`, `FILLM` and `PRCLM`; a
mailbox's `BUFQUO` charged to its creator's `BYTLM`; `$CREPRC`'s quota
list; CPU time and the counts for `SHOW PROCESS/ACCOUNTING`.
