# ADR-0021 — Condition handlers are found along the FP chain and run in the mode that signaled, from code in the vector

Oct 5, 2026 · @Marko Mikulicic

Proposed. A routine's condition handler is the longword at `0(FP)` in its
frame, as on the VAX. An exception in an outer mode no longer ends the
image: the executive copies the PAL's frame, the registers and VMS's
signal and mechanism arrays below the stack of the mode that took it, and
`REI`s to `EXE$SRCHANDLER` in the vector, which calls the handlers in that
mode. `LIB$SIGNAL` calls the same search. `.ENTRY` records its register
mask in the frame, so that `$UNWIND` can return from any frame. A
condition no handler takes is written with `$PUTMSG` and, if severe, ends
the image with `$EXIT`.

## Context

[ADR-0005](0005-access-modes-are-threads.md) made an outer mode's faults
exceptions the executive handles, and listed "condition handlers in place
of exiting on an exception" as a follow-up. `EXE$ACVIOLAT` and
`EXE$OPCDEC` wrote the fault on the console and called `EXE$IMGEXIT`,
which skipped the image's exit handlers. A program could not catch an
access violation, and no program could signal a condition.

On VMS, a routine establishes a handler by writing its address at
`0(FP)`, or with `LIB$ESTABLISH`. An exception, or `LIB$SIGNAL`, builds
a signal array (the condition, its arguments, the PC and the PSL) and a
mechanism array (the establisher's frame, its depth, R0 and R1). The
handlers are then called from the signaling frame outward, in the mode
that signaled. A handler continues, resignals, or calls `$UNWIND` to
return from frames. The catch-all handler writes the message of a
condition nobody took, and exits if it is severe.

vmacro's frame already had the handler slot at `0(FP)`, with the
caller's FP at `16(FP)`. Its `RET` is static, though: each `RET` knows its
routine's entry mask at assembly time, and nothing in the frame says
which registers it saved.

## Decision

1. **The handler is at `0(FP)`.** `.ENTRY` clears it, as before.
   `LIB$ESTABLISH` and `LIB$REVERT` set and clear the caller's.
2. **`.ENTRY` keeps its mask at `40(FP)`**, the frame's unused slot, so
   code that never ran a routine's `RET` can still restore what that
   routine saved.
3. **Exceptions are reflected to the mode that took them.**
   `EXE$ACVIOLAT` and `EXE$OPCDEC`, in kernel mode, write the following
   below that mode's stack: the registers, the handlers' argument list,
   the mechanism and signal arrays, and a copy of the PAL's frame. They
   then point the frame at `EXE$SRCHANDLER` and `REI`. If the stack can't
   take them, the image exits with the condition, as it used to.
4. **The search runs in the mode that signaled, from the vector.**
   `EXE$SIGNAL` walks the saved FPs out from the signaling frame. It
   stops at an FP of 0, which `EXE$USRSTART`, `EXE$CLISTART` and
   `EXE$ASTDISP` set, or at a frame the mode can't read. It calls each
   handler it finds. `EXE$SRCHANDLER` continues an exception with a
   user-mode `REI` from the copied frame, at the signal array's PC.
   `LIB$SIGNAL` continues by returning.
5. **`$UNWIND` runs in the caller's mode**, as `SYS$UNWIND` in the vector,
   without `CHMK`. It finds `EXE$SIGNAL`'s frame by the return address of
   the handler's call, and records the last frame to remove. Once the
   handler returns, `EXE$SIGNAL` loads every register each removed frame
   saved, by its mask, from the innermost out. It then returns from the
   last frame to its caller, or to `newpc`, with R0 and R1 from the
   mechanism array.
6. **The catch-all is `EXE$CATCHALL`.** It writes the message with
   `$PUTMSG`, also in the vector, which fills `$GETMSG`'s FAO directives
   from the signal array. If the condition is severe, it calls `$EXIT`
   with `STS$M_INHIB_MSG` set. The exit therefore runs the image's exit
   handlers, and DCL does not write the message a second time.
7. **The vector is two psects.** Modules after `syssrv.mar` add code to
   `EXEC$VECTOR`. `EXEC$VECTOREND`, a psect of its own, marks the end of
   the vector, so it stays past whatever they add.

## Alternatives considered

| Option | Why not |
| --- | --- |
| Search for handlers in kernel mode and call them with `CHMx` or an AST | A handler must run in the mode of the code it handles, with that mode's stack and registers, and `$UNWIND` must return into that code. Driving that from kernel mode needs a mode change per handler. |
| Unwind by patching each frame's return address so that its own `RET` runs, as VAX `$UNWIND` does | It only works for frames that reach their `RET`. A frame that took an exception never does, and vmacro's `RET` is a sequence of instructions inlined in each routine, so there is no single place to jump to. |
| Describe frames in unwind tables built by vlink | That is the Alpha and Itanium approach and is more complete, but a mask in the frame costs one instruction per call and needs no linker changes. |
| `LIB$SIGNAL` removes its own frame, as VMS's does, so that a signal looks like an exception | Starting the search at its caller's frame gives handlers the same depths, and unwinding skips `LIB$SIGNAL`'s frame anyway. |
| Keep the console report in the executive and only add `LIB$SIGNAL` | Programs, and the C that the sockets library will bring, could still not catch faults, and exceptions would still skip exit handlers. |

## Consequences

**What gets harder.**
- Every `.ENTRY` has one more instruction.
- An exception needs 208 bytes of the faulting mode's stack. If the stack
  overflowed, there is no room, and the image exits as it used to.
- For a reserved PAL call, the signal's R7 is the PAL's function code,
  because vmacro keeps the real R7 in a scratch register. That register
  comes back only if the handler continues at the same PC.
- The vector holds more code that outer modes run, so it must use
  nothing in S0 outside the vector, and must keep no writable data.

**What stays easy.**
- Handlers, `LIB$SIGNAL`, `$UNWIND` and the arrays are VMS's, so MACRO-32
  code that handles conditions reads as it does on VMS.
- An image that faults now runs its exit handlers, and DCL's `$STATUS`
  has the condition with `STS$M_INHIB_MSG`, as on VMS.

**Follow-ups:** calling the handlers of removed frames with `SS$_UNWIND`;
skipping frames already searched when a handler faults; primary,
secondary and last-chance exception vectors; `$PUTMSG`'s action routine,
multiple messages and `SYS$ERROR`; `LIB$SIGNAL` for errors from
`CLI$` routines (the ponytail in `lib/cli.mar`); arithmetic traps.
