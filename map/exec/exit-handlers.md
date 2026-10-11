+++
title = "Exit handlers and rundown"
summary = "$EXIT, $DCLEXH and $FORCEX; image and process rundown"
aka = ["$EXIT", "$DCLEXH", "$CANEXH", "$FORCEX", "image rundown", "process rundown", "EXE$IMGRUNDOWN", "EXE$RUNDOWN", "final status", "SS$_FORCEDEXIT"]
kind = "core"
intent = "goal"
research = "done"
milestone = "m0"
refs = ["docs/design/0002-executive-processes.md", "docs/adr/0006-cli-in-p1-runs-images-in-its-process.md", "docs/adr/0013-qio-irps-and-drivers.md", "docs/adr/0014-ctrlc-ctrly-asts.md", "docs/adr/0019-mailboxes.md", "vms/exec/process.mar"]

[[feature]]
id = "exit-handlers"
title = "$DCLEXH and $CANEXH; $EXIT calls them in the caller's mode, last declared first"
status = "done"
milestone = "m0"

[[feature]]
id = "forcex"
title = "$FORCEX: a user mode AST that calls $EXIT"
status = "done"
milestone = "m0"
refs = ["docs/adr/0014-ctrlc-ctrly-asts.md"]

[[feature]]
id = "image-rundown"
title = "Image rundown: the image's I/O cancelled, then its pages, timers, flags, ASTs, files"
status = "done"
milestone = "m0"
refs = ["docs/adr/0006-cli-in-p1-runs-images-in-its-process.md", "docs/adr/0013-qio-irps-and-drivers.md"]

[[feature]]
id = "process-rundown"
title = "Process rundown in its own context, then the termination message"
status = "done"
milestone = "m0"
needs = ["//exec/mailboxes:termination"]
refs = ["docs/adr/0019-mailboxes.md"]

[[feature]]
id = "cli-resources"
title = "Rundown keeps the command interpreter's timers and common event flags, not only its channels"
status = "todo"
note = "process.mar ponytail; ADR-0006"

[[feature]]
id = "mode-names"
title = "User mode logical names deleted at image rundown"
status = "todo"
needs = ["//exec/logical-names:access-modes"]

[[needs]]
on = "//exec/processes"
why = "It ends an image or a process"

[[needs]]
on = "//exec/asts"
why = "$FORCEX and $DELPRC are ASTs; rundown flushes ACBs"

[[needs]]
on = "//exec/qio"
why = "Cancels the image's or the process's I/O first"

[[needs]]
on = "//exec/memory"
why = "Deletes P0 at image rundown, P1 at process rundown"

[[needs]]
on = "//exec/timers"
why = "Removes timer queue entries"

[[needs]]
on = "//exec/event-flags"
why = "Dissociates common event flag clusters"

[[needs]]
on = "//rms/core"
why = "RMS$RUNDOWN closes the files"
+++

When a VMS image calls `$EXIT`, or returns, or a severe condition ends
it, the system first calls the exit handlers it declared with `$DCLEXH`,
each once, newest first, in the access mode that declared it, with the
final status. Then image rundown gives back what the image held: its I/O
is cancelled, user mode ASTs, locks, logical names and channels go, its
P0 pages are deleted, and the command interpreter gets control with the
status. `$FORCEX` makes another process's image exit this way. Process
rundown, for `$DELPRC` or a process with no command interpreter, gives
back the rest and sends the termination message to the creator's
mailbox. RMS closes files and writes their last blocks on the way.

vaxpunk's are in `vms/exec/process.mar`. `$EXIT` hands each handler to
`SYS$EXIT` in the vector, which calls it in the caller's mode. Image
rundown cancels the image's I/O before P0 goes
([ADR-0013](../../docs/adr/0013-qio-irps-and-drivers.md)) and keeps the
channels and files the command interpreter had when the image started
([ADR-0006](../../docs/adr/0006-cli-in-p1-runs-images-in-its-process.md)).
Process rundown runs in the process's own context, since only it can
reach its P0 and P1, and the swapper frees the PCB and kernel stack
after. Locks join rundown with the lock manager (`//exec/lock-manager:rundown`).
