# ADR-0006 — The command interpreter lives in P1 in supervisor mode and runs images in its own process

Oct 1, 2026 · @Marko Mikulicic

Accepted. A command interpreter, DCL, is an image linked in P1. A process
whose image is one keeps it there, in supervisor mode, for its whole life,
and runs each image the user asks for in its own P0, in user mode. When an
image exits, the executive runs it down and calls the command interpreter
again, on an empty stack, with the image's status.

## Context

Until now a process ran one image, and its end was the process's: there
was no way to run an image but `$CREPRC`, and only `STARTUP` ran, from
`EXEC$START`. [ADR-0005](0005-access-modes-are-threads.md) gave each
process supervisor mode and left it "for a command interpreter to run in".

On VMS, `LOGINOUT` maps DCL into P1 of the process it logs in, where it
runs in supervisor mode. `RUN` activates the image into P0 and calls it in
user mode; `$EXIT` runs the image down and control comes back to DCL,
whose P1 data, symbols and command procedures included, outlives every
image. User mode can't touch DCL's pages, and an image that faults ends
the image, not the process.

## Decision

1. **An image linked in P1, from `VA$C_CLI` up to the mode stacks, is a
   command interpreter.** The image activator maps it supervisor readable,
   its writable sections supervisor writable, and keeps its transfer
   address in the PCB. ponytail: the link address says what an image is,
   since there is no `LOGINOUT` or UAF to say which command interpreter a
   process gets.
2. **The executive calls it in supervisor mode** through `EXE$CLISTART`
   in the system service vector, with a status: `SS$_NORMAL` when the
   process starts, then the status of each image that exits. If it
   returns, the process is deleted, which is `LOGOUT`.
3. **`$IMGACT` runs an image in the current process.** It runs down the
   image there was, keeping the channels the process has, activates the
   new one in P0 and calls it in user mode. It returns only if the image
   can't be activated. VMS's `$IMGACT` only activates; here it also calls
   the image, since the command interpreter would only `REI` there.
4. **`$EXIT` in a process with a command interpreter is image exit.** The
   executive runs the image down (its P0 pages, timer queue entries, common
   event flag clusters and the channels it assigned) and calls the command
   interpreter again on an empty kernel and supervisor stack.
5. **The console's process runs DCL.** `EXEC$START` creates `SYSTEM` with
   `DCL.EXE`, as logging in on the console gives a process `DCL`, and the
   test programs run from its prompt.

## Alternatives considered

| Option | Why not |
| --- | --- |
| DCL as a user-mode image of its own process, running each command's image in a subprocess and waiting for it | Not how VMS works: images couldn't share the process's channels, logical names and privileges with DCL, a subprocess costs a PCB and a context per command, and DCL would need termination mailboxes to learn an image's status. |
| DCL in P0 next to the images | An image could overwrite it, and images couldn't link at `0x10000`. |
| DCL in user mode in P1 | User mode could write DCL's data; supervisor mode is what it is for (ADR-0005). |
| Return from `$IMGACT` into the command interpreter, and have `$EXIT` resume it where it called the image | The command interpreter's stack and registers would have to survive the image, and every way out of an image (`$EXIT`, an exception, `$DELPRC`) would have to find that frame. Calling it afresh needs nothing saved: what DCL keeps is in its P1 data. |
| `$CREPRC` taking the command interpreter's name | VMS's `$CREPRC` has no such argument, and `LOGINOUT` is a long way off. |

## Consequences

**What gets harder.**
- DCL keeps no state on its stack across an image: each call starts at its
  transfer address, and what it remembers is in its data.
- Image rundown has to tell the image's resources from the command
  interpreter's. Today only channels: those the process had when `$IMGACT`
  ran survive. ponytail: timers and common event flags are all run down;
  DCL uses neither.
- Nothing interrupts an image that never exits: there is no CTRL/Y yet,
  which needs a way to make the console's process exit its image from
  outside, like `$DELPRC`'s.

**What stays easy.**
- An image is the same whether it runs in a process of its own or under
  DCL: it returns or calls `$EXIT`, and an exception ends it.
- A process without a command interpreter still ends with its image.

**Follow-ups:** CTRL/Y and `$FORCEX`; message texts for statuses; DCL
symbols, qualifiers and command procedures; `LOGINOUT` choosing the
command interpreter.
