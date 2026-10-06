# ADR-0014 — CTRL/C and CTRL/Y are ASTs enabled with IO$_SETMODE, and DCL's CTRL/Y AST stops the image

Oct 3, 2026 · @Marko Mikulicic

Proposed. A process enables a CTRL/C or CTRL/Y AST on the console with
`IO$_SETMODE` and `IO$M_CTRLCAST` or `IO$M_CTRLYAST`. When the key comes,
the terminal driver queues each AST enabled for it, which disables it.
CTRL/C that no AST is enabled for is CTRL/Y. DCL enables a supervisor
mode CTRL/Y AST; delivered on top of the image, it is where DCL prompts
while the image is stopped, and `CONTINUE` returns from it. This
replaces `EXE$CTRLY`, `$CONTINUE` and `PCB$L_CTRLY`, and supersedes
[ADR-0010](0010-ctrly-calls-the-cli-on-top-of-the-image.md). `$CANCEL`
ends a channel's requests with `SS$_ABORT` and disables its CTRL/C and
CTRL/Y ASTs; `$FORCEX` queues a user mode AST that calls `$EXIT`.

## Context

[ADR-0010](0010-ctrly-calls-the-cli-on-top-of-the-image.md) stopped an
image for CTRL/Y before there were ASTs. The terminal driver picked the
process with a command interpreter that ran an image, set
`PCB$V_CTRLY` there, and the process checked that flag on each way back
to user mode and at the end of each wait, in `EXE$CTRLYCHK` and
`SCH$WAIT`. `EXE$CTRLY` saved the registers on the kernel stack and
called DCL afresh below them, and `$CONTINUE`, a service VMS doesn't
have, put them back. It said so itself: the right end state was a
supervisor mode CTRL/Y AST, as on VMS.

[ADR-0011](0011-asts-on-the-kernel-stack.md) gave the executive ASTs,
delivered by the same means: the registers saved on the kernel stack, the
routine called in its mode below them, `$ASTEXIT` returning. The PAL
requests delivery on any `REI` to a mode an AST waits for, and
`SCH$WAIT` delivers when a wait ends, which are the points
`EXE$CTRLYCHK` checked. [ADR-0013](0013-qio-irps-and-drivers.md) gave the
console a driver with an FDT routine and IRPs on the device's queue, and
listed `$CANCEL`, `IO$_SETMODE` with CTRL/C and CTRL/Y ASTs, and
`$FORCEX` as what it unblocked.

On VMS, `IO$_SETMODE!IO$M_CTRLYAST` on a terminal channel enables an AST
for the next CTRL/Y: p1 is its routine, p2 its parameter, p3 its access
mode, and the AST, once delivered, must be enabled again. DCL keeps one
enabled in supervisor mode. Its AST interrupts the image, in user mode
or in a wait, and DCL reads commands inside it: `CONTINUE` returns, and a
command that runs an image runs the stopped one down. CTRL/C works the
same with `IO$M_CTRLCAST`, and is taken as CTRL/Y when no CTRL/C AST is
enabled. `$CANCEL` ends a channel's requests with `SS$_ABORT`, and the
terminal driver's cancel also drops the channel's CTRL/C and CTRL/Y ASTs.
`$FORCEX` makes another process's image exit, with a user mode AST that
calls `$EXIT`.

## Decision

1. **`IO$_SETMODE` with `IO$M_CTRLCAST` or `IO$M_CTRLYAST`** on the
   console (`TT$FDT`) puts the AST in an IRP, from `IOC$ALLOCIRP`, with
   p1, p2, and p3 or the caller's mode, whichever is the outer one, and
   queues it on `TTY$Q_CTRLC` or `TTY$Q_CTRLY`, after taking off the one
   the channel had. p1 = 0 only takes it off. The request is completed
   at once, with a second IRP.
2. **CTRL/C and CTRL/Y** empty the type-ahead buffer in the receive
   interrupt, as CTRL/Y did, and leave a flag for `TTY$IOPOST`. There the
   driver echoes `*CANCEL*` or `*INTERRUPT*` and queues every AST on the
   key's queue to its process with `SCH$QAST`, which takes it off: an AST
   is enabled once. CTRL/Y first completes the console reads of each
   process it goes to with `SS$_CONTROLY`, as ADR-0013 did. CTRL/C with
   no AST on `TTY$Q_CTRLC` is CTRL/Y; CTRL/Y with none does nothing.
3. **DCL enables a supervisor mode CTRL/Y AST**, `CTRLY`, when it starts,
   before each image it runs, and when it continues one. Delivered while
   an image runs, the AST ends the procedures running and goes to DCL's
   command loop, keeping its frame pointer: `CONTINUE` enables the AST
   again and returns from it, and `$ASTEXIT` takes the image back where
   it was. Delivered with no image, the AST enables itself again and
   returns, and DCL ends its procedures at the next command; the read
   CTRL/Y ended gives it an empty line.
4. **`$IMGACT` runs the stopped image down as any other**: image rundown
   flushes the ASTs and forgets the AST routines running, DCL's included,
   and `EXE$USRENTRY` empties the kernel stack. It now also empties the
   supervisor stack, by setting the HWPCB's `SSP` to its top, so that DCL
   frames left from a CTRL/Y AST don't pile up. Its check for a stopped
   image, to keep `PCB$L_CLICHANS` and `PCB$L_CLIFILES`, is a supervisor
   mode AST routine running, `PCB$B_ASTACT`.
5. **`$CANCEL chan`** calls `IOC$CANCEL` with `SS$_ABORT`: the channel's
   requests the UCBs hold are completed with it, those done already are
   left, and its CTRL/C and CTRL/Y ASTs are taken off their queues and
   freed. Rundown and `$DASSGN` call it with 0, which frees the requests
   unfinished, as before.
6. **`$FORCEX pidadr, prcnam, code`** queues the process a user mode AST
   whose routine is `SYS$EXIT` and whose parameter is code, or
   `SS$_FORCEDEXIT` for 0. It comes when the image next runs in user
   mode, and ends a wait.

## Alternatives considered

| Option | Why not |
| --- | --- |
| Keep `EXE$CTRLY` and `$CONTINUE`, and add only CTRL/C | Two mechanisms for one thing: a flag checked on every return to user mode, beside ASTs that are delivered at the same points, and a service VMS doesn't have |
| Keep DCL's CTRL/Y AST enabled while DCL prompts inside it | A second CTRL/Y would end DCL's own read and queue an AST that, blocked while the first runs, stops the image again as soon as `CONTINUE` returns |
| Enable DCL's CTRL/Y AST only while an image runs | CTRL/Y couldn't stop a procedure looping in DCL, which it now does; and the AST may come just as an image exits, so DCL has to tell the two cases apart anyway |
| `CONTINUE` returns by calling `$ASTEXIT` | DCL would need the AST's frame anyway to unwind its own; a `RET` from the AST routine is what VMS's DCL does, and `EXE$ASTDISP` calls `$ASTEXIT` after it |
| The ASTs in the console's UCB, as VMS's `UCB$L_TT_CTRLY` lists | One terminal; the line being read is kept in `ttdriver.mar` for the same reason |
| An ACB with a channel field for the enabled ASTs | An IRP already has the channel and the PID, so `IOC$CANCEL`'s `CANCELQ` takes them off with the requests, and is an ACB when queued |
| `$CANCEL` frees the requests, as rundown does | The caller is still there and waits for the I/O status block or the event flag |
| `$FORCEX` deletes the process, or runs its image down from a kernel mode AST | `$EXIT` in the process already does rundown right, with or without a command interpreter, and VMS's `$FORCEX` is that user mode AST |

## Consequences

**What gets harder.**
- DCL tracks whether an image runs, since its CTRL/Y AST can come with
  none, or as an image exits: the AST then takes DCL's prompt for a
  stopped image's, which `CONTINUE` leaves as if it were.
- A failed `$IMGACT` from inside the CTRL/Y AST leaves DCL in it, with the
  image gone; DCL forgets the AST, and its frames on the kernel stack stay
  until the next image runs. One whose name is too long fails before it
  runs the old image down, which then can't be continued either.
- DCL's and the image's reads share event flag 0: re-enabling the CTRL/Y
  AST sets it, which `$QIOW`'s IOSB check covers.
- A `$FORCEX` for an image CTRL/Y stopped waits until `CONTINUE`, and one
  for a process with no image is dropped at the next image's activation,
  with the other user mode ASTs.
- ponytail: one AST per channel and key. Without `IO$M_CTRLCAST` or
  `IO$M_CTRLYAST`, `IO$_SETMODE` sets the terminal's characteristics,
  which `IO$_SENSEMODE` returns.

**What stays easy.**
- The image doesn't know: every register is as it was after `CONTINUE`,
  as ADR-0010 kept them.
- A program that wants CTRL/C enables its own AST, as on VMS, and `$CANCEL`
  ends a read from there.

**Follow-ups:** DCL's `STOP` and `SET NOCONTROL`; `$DCLEXH` exit handlers,
which `$FORCEX` runs on VMS; mailboxes for `$FORCEX`'s process termination messages.
