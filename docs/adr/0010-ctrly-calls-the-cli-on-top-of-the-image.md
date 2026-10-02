# ADR-0010 — CTRL/Y calls the command interpreter on top of the stopped image, and $CONTINUE goes back to it

Oct 2, 2026 · @Marko Mikulicic

Accepted. CTRL/Y on the console stops the image the console's command
interpreter runs, where it is, and calls the command interpreter in
supervisor mode with `SS$_CONTROLY`, on the process's kernel stack as it
is, so the image's registers and the service or interrupt it was in stay
there. DCL's `CONTINUE` calls `$CONTINUE`, which goes back to the image;
a command that runs another image runs the stopped one down.

## Context

[ADR-0006](0006-cli-in-p1-runs-images-in-its-process.md) left an image
that never exits holding the console, and listed CTRL/Y as a follow-up.

On VMS, the terminal driver echoes `*INTERRUPT*` and delivers DCL's
CTRL/Y AST, a supervisor mode AST. The process enters supervisor mode on
top of the image's user mode, so the image is stopped wherever it was,
even in a wait, which the AST interrupts. DCL reads commands inside the
AST: `CONTINUE` returns from it, and the image goes on; `RUN` and the other
commands that run an image first run the stopped one down.

vaxpunk has no ASTs yet. The PAL moves the CPU between a process's mode
threads by copying the registers ([ADR-0005](0005-access-modes-are-threads.md)),
so the image's registers are in kernel mode's while the process is there,
and kernel mode can `REI` out to supervisor mode from wherever it is: the
next `CHMK` pushes its frame below the stack pointer that `REI` left, and
what was above stays. DCL is called afresh each time, and returning from
it deletes the process (ADR-0006).

## Decision

1. **CTRL/Y in the console's receive interrupt** empties the type-ahead
   buffer and leaves a flag for `TTY$IOPOST`. There it goes to the process
   with a command interpreter, if it runs an image CTRL/Y hasn't stopped
   yet: the console echoes `*INTERRUPT*`, `PCB$V_CTRLY` is set in its PCB,
   and its wait ends, if it waits. Otherwise CTRL/Y does nothing.
2. **The process serves it at a point where its image can be left**:
   going back to user mode from `TTY$IOPOST`, from `SCH$RESCHED` or from a
   system service, or ending a wait in `SCH$WAIT`, whose callers look
   again at what they waited for. `$HIBER` now loops, as `SCH$EFWAIT` and
   the console read already did, and `$WAKE` always leaves a wakeup for it
   to take.
3. **`EXE$CTRLY` saves the registers and IPL on the kernel stack**, keeps
   that stack pointer in `PCB$L_CTRLY`, and `REI`s to `EXE$CLISTART` below
   it, in supervisor mode on an empty supervisor stack, with
   `SS$_CONTROLY`. DCL prompts as it does after any image.
4. **`$CONTINUE`**, a vaxpunk service, puts the stack pointer back from
   `PCB$L_CTRLY`, then the IPL and registers, and returns from
   `EXE$CTRLY`: the image goes on from where it stopped. Without a stopped
   image it does nothing, as VMS's `CONTINUE`.
5. **Image rundown forgets the stopped image**: `$IMGACT` from the
   command interpreter, which empties the kernel stack when it calls the
   new image, runs the old one down, its channels and files included,
   since `PCB$L_CLICHANS` and `PCB$L_CLIFILES` still hold what the process
   had when it ran.

## Alternatives considered

| Option | Why not |
| --- | --- |
| ASTs first, then a supervisor CTRL/Y AST as VMS has | The right end state, but AST delivery, `$DCLAST` and `$SETAST` are a subsystem of their own; this is the one AST the console needs now, and the save area on the kernel stack is what an AST frame would hold |
| Save only the user-mode registers in the PCB, and stop the image only in user mode | A wait inside a service, `$HIBER` or a console read, would have to be unwound and restarted, and the image's registers found in the service's call frames |
| CTRL/Y ends the image, as `$FORCEX` would | Not VMS: `CONTINUE` couldn't go back, and DCL's `STOP` is what ends it there |
| DCL returns from the CTRL/Y call to continue | DCL's return is `LOGOUT` (ADR-0006), and `LOGOUT` would have to become `$DELPRC` |
| A field of the terminal, `IO$M_CTRLYAST` on DCL's channel, to say who gets CTRL/Y | There is no terminal UCB yet; the console has one command interpreter, `SYSTEM`'s |

## Consequences

**What gets harder.**
- Kernel code that waits through `SCH$WAIT` must expect the wait to end
  without what it waited for, and hold nothing that another image's
  rundown would leak, since the image may never come back.
- The command interpreter's commands run on the kernel stack below the
  stopped image's, so its depth adds to theirs.
- A read the image had on the console isn't cancelled, as VMS's
  `SS$_CONTROLY` would: after `CONTINUE` it goes on reading. ponytail.
- Channels the command interpreter assigns while an image is stopped go
  with that image. DCL assigns its one on its first call.

**What stays easy.**
- The image doesn't know: it goes on with every register as it was.
- ASTs, when they come, can deliver CTRL/Y the same way and replace
  `$CONTINUE` with the return from the AST.

**Follow-ups:** ASTs and a CTRL/Y AST enabled with `IO$M_CTRLYAST`; CTRL/Y
at the DCL prompt; CTRL/C; DCL's `STOP` and `EXIT`; `$FORCEX`.
