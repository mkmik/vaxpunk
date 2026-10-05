# ADR-0019 — A mailbox is a unit of its own, MBnn, whose writes are done at once, and process deletion writes the termination message to it

Oct 5, 2026 · @Marko Mikulicic

Proposed. `$CREMBX` makes a mailbox as a UCB from pool, `MBnn`, on a list
of its own, as `BGA0:` clones network units, and assigns a channel to it;
a logical name for it goes in the system table. A write queues the
message on the UCB and is done at once; a read takes the oldest message
or waits on the UCB's queue. A temporary mailbox goes with its last
channel, a permanent one once `$DELMBX` has made it temporary. `$CREPRC`'s
`mbxunt` is kept in the PCB, and process rundown writes the process's
termination message, `MSG$_DELPROC` in `$ACCDEF`'s layout, to that
mailbox.

## Context

[ADR-0013](0013-qio-irps-and-drivers.md) and
[ADR-0014](0014-ctrlc-ctrly-asts.md) listed mailboxes as a follow-up, the
second for `$FORCEX`'s process termination messages. `$CREPRC` took
`mbxunt` and ignored it, and `$ASSIGN`'s `mbxnam` too. A process that
starts another had no way to learn when it ends, or with what status,
but to poll `$GETJPI`; two processes had no way to pass data but common
event flags.

On VMS a mailbox is a pseudo-device, `MBAn`, with a UCB the mailbox
driver serves. `$CREMBX prmflg, chan, maxmsg, bufquo, promsk, acmode,
lognam, flags` makes one, or finds the one `lognam` already names, and
assigns a channel; the name goes in `LNM$TEMPORARY_MAILBOX` or
`LNM$PERMANENT_MAILBOX`. `IO$_WRITEVBLK` queues a message and, without
`IO$M_NOW`, waits until a reader takes it; `IO$_READVBLK` takes the
oldest, or waits, or with `IO$M_NOW` ends with `SS$_ENDOFFILE`. The IOSB
has the size and the writer's PID. When a process with a termination
mailbox is deleted, `DELETE` in `SYSDELPRC` assigns a channel to
`MBnnnn` and writes an `ACC$K_TERMLEN`-byte message with
`IO$_WRITEVBLK!IO$M_NOW`: the type, `MSG$_DELPROC`, then the final status
`$EXIT` left in `CTL$GL_FINALSTS`, the PID, the time, accounting, and the
owner's PID.

Here devices are UCBs in `qio.mar`, four-character names, and a network
unit, `BGnn`, is a UCB `BGA0:` clones into pool, which counts its
channels ([ADR-0016](0016-tcpip-component-and-bga0.md)). There are two
logical name tables, the process's and the system's.

## Decision

1. **A mailbox is a UCB from pool, `UCB$K_MBLENGTH`, named `MBnn`**, 1-99,
   the next number no mailbox has, on `MB$GL_UNITS`. It has the network
   units' link, channel count and unit number, and a queue of messages,
   the longest message it takes and the bytes it may still hold.
   `$ASSIGN` finds it by name after the network units. `IOC$ASSIGN`, which
   `$ASSIGN` and `$CREMBX` share, counts a channel for any UCB that isn't
   one of `qio.mar`'s.
2. **`$CREMBX`** translates `lognam` as `$ASSIGN` would and, if that is a
   mailbox, assigns a channel to it. Else it makes one, with `maxmsg`,
   256 for 0, and `bufquo`, 1056 for 0, VMS's `DEFMBXMXMSG` and
   `DEFMBXBUFQUO`, permanent for `prmflg` = 1, and makes `lognam` a name
   in `LNM$SYSTEM_TABLE` whose equivalence is `_MBnn:`. `promsk`,
   `acmode` and `flags` are ignored.
3. **A write is done at once.** `MB$FDT`'s `IO$_WRITEVBLK` copies the
   message into a block of pool on the UCB's queue of messages, and
   completes the request: `SS$_MBTOOSML` for one over `maxmsg`,
   `SS$_MBFULL` for one the mailbox has no room left for.
   `IO$_READVBLK` is a buffered read that waits on the UCB's queue; each
   message goes to the oldest read, with its size and the writer's PID in
   the IOSB, `SS$_BUFFEROVF` if it was cut to fit. With `IO$M_NOW` and no
   message, the read ends with `SS$_ENDOFFILE`.
4. **A mailbox goes with its last channel**, in `MB$CANCEL`, which
   `IOC$CANCEL` calls beside `NET$CANCEL`: its messages, the logical names
   whose equivalence is its name, and its UCB. A permanent one stays with
   none, until `$DELMBX` makes it temporary, which also deletes its names.
   `$CANCEL` ends a channel's reads with `SS$_ABORT`, as other devices'.
5. **The termination message.** `$CREPRC` keeps `mbxunt` in
   `PCB$L_TMBU`, `$EXIT` keeps its status in `PCB$L_FINALSTS`, and
   `EXE$RUNDOWN`, once the process has given back the rest, writes
   `$ACCDEF`'s 84 bytes to mailbox `MBnn` with `MB$SEND`, in the
   process's own context: `MSG$_DELPROC`, the final status, the PID, the
   system time and the owner's PID, the rest 0. A mailbox that is gone or
   full doesn't get it.
6. **`$GETDVI` on a mailbox's channel** has `DVI$_DEVNAM`,
   `DVI$_DEVCLASS`, `DC$_MAILBOX`, and `DVI$_UNIT`, which is what a
   creator passes as `mbxunt`.

## Alternatives considered

| Option | Why not |
| --- | --- |
| A write waits until a reader takes the message, as VMS's without `IO$M_NOW` | A write IRP on the UCB beside the message, and `IOC$CANCEL` ending both; nothing here needs the wait yet, and the executive's own write mustn't wait anyway |
| A write waits for room, as VMS's in resource wait | No resource waits in the scheduler; `SS$_MBFULL` is what VMS returns with resource wait mode off |
| Fixed mailbox UCBs in `qio.mar`, as the disks' | A fixed number, and every one in `IOC$CANCEL`'s loop; units from pool are what the network units already are |
| `MBAnnnn`, VMS's names | Device names here are four characters, `DDCU`; VMS V4's `SYSDELPRC` names them `MBnnnnn` too |
| The names in `LNM$TEMPORARY_MAILBOX` and `LNM$PERMANENT_MAILBOX`, which are `LNM$JOB` and `LNM$SYSTEM` | There is no job table; another process must find a temporary mailbox by name, so both go in the system table |
| A pointer to the logical name block in the UCB, as VMS's `UCB$L_LOGADR` | `$CRELNM` may replace the block; deleting the names whose equivalence is the mailbox's name needs no pointer to keep right |
| The termination message through `$ASSIGN` and `$QIO` in the process, as `SYSDELPRC` does | Rundown has given back the channels by then; `MB$SEND` is the write without a channel or an IRP |
| The termination message in `$DELPRC`, in the deleter's context | A process that `$EXIT`s deletes itself; rundown is the one place every deletion goes through |

## Consequences

**What gets harder.**
- Only 99 mailboxes at once, and `$CREMBX` fails with `SS$_INSFMEM` past
  them.
- ponytail: no `BUFQUO` quota on the process; a mailbox's `bufquo` is
  only its own limit, and any process may write to any mailbox: no
  protection mask. Permanent mailboxes take `PRMMBX`, temporary ones
  `TMPMBX` (PRD-0003).
- A reader that never reads holds the writer's messages in pool until the
  mailbox goes.
- The termination message has 0 for the job, account, user name, CPU
  time, the counts and the login time, which nothing keeps yet, and a
  process `$DELPRC` deletes before any `$EXIT` has 0 for its status.

**What stays easy.**
- A mailbox is a device: `$QIO`, `$QIOW`, ASTs, event flags, `$CANCEL`
  and rundown work on it as on the others.
- A process that starts another can read its end, with its status, from
  a mailbox, as DCL's `SPAWN` and a batch queue need.

**Follow-ups:** a write that waits for its reader, and `IO$_WRITEOF`;
`IO$_SETMODE` attention ASTs; `$ASSIGN`'s `mbxnam`; `DVI$_DEVNAM` and
`$DEVICE_SCAN` finding mailboxes by name; DCL's `SPAWN` and `RUN/DETACH`,
which use the termination message; quotas and protection.
