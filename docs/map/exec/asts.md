+++
title = "ASTs"
summary = "Asynchronous system traps: calls a process gets in one of its access modes"
aka = ["AST", "ACB", "$DCLAST", "$SETAST", "$ASTEXIT", "ASTSR", "ASTEN", "ASTLM", "kernel mode AST", "SCH$QAST", "astdel.mar"]
kind = "core"
intent = "goal"
research = "done"
milestone = "m0"
refs = ["docs/adr/0011-asts-on-the-kernel-stack.md", "docs/adr/0014-ctrlc-ctrly-asts.md", "docs/adr/0013-qio-irps-and-drivers.md", "docs/design/0002-executive-processes.md", "vms/exec/astdel.mar", "docs/prd/0003-multi-user-vms.md"]

[[feature]]
id = "delivery"
title = "AST queues by mode; delivery when IPL and the mode allow, and when a wait ends"
status = "done"
milestone = "m0"
refs = ["docs/adr/0011-asts-on-the-kernel-stack.md"]

[[feature]]
id = "services"
title = "$DCLAST, $SETAST and $ASTEXIT"
status = "done"
milestone = "m0"

[[feature]]
id = "kernel-asts"
title = "Kernel mode ASTs that run an executive routine: deletion, I/O completion"
status = "done"
milestone = "m0"
refs = ["docs/adr/0011-asts-on-the-kernel-stack.md", "docs/adr/0013-qio-irps-and-drivers.md"]

[[feature]]
id = "ctrl-c-y"
title = "CTRL/C and CTRL/Y as ASTs; DCL's CTRL/Y AST stops the image"
status = "done"
milestone = "m0"
refs = ["docs/adr/0014-ctrlc-ctrly-asts.md"]

[[feature]]
id = "astdel-ipl"
title = "IPL$_ASTDEL around kernel code that mustn't be deleted halfway"
status = "todo"
milestone = "m1"
note = "PRD-0003 tier 2 #13; ADR-0011 consequences"

[[feature]]
id = "astlm"
title = "ASTLM: the AST quota charged and checked"
status = "todo"
milestone = "m1"
note = "PRD-0003 tier 1 #4; with t8y:/exec/quotas#asts-timers, which needs this component, so no need back"

[[feature]]
id = "arglist"
title = "VMS's full AST argument list: the parameter, then R0, R1, PC and PSL"
status = "todo"
note = "PRD-0003 tier 3 #34"

[[needs]]
on = "/exec/scheduler"
why = "Queueing an AST ends the process's wait"

[[needs]]
on = "/exec/pool"
why = "ACBs come from nonpaged pool"

[[needs]]
on = "/exec/ipl-sync"
why = "Delivered by the IPL 2 software interrupt; queues change at IPL$_SYNCH"

[[needs]]
on = "/platform/pal"
why = "The PAL reads ASTSR and ASTEN in the HWPCB and requests IPL 2"
+++

An AST is a call a process gets on top of whatever it was doing, in one
of its four access modes. Timers, I/O completion, lock grants, CTRL/C and
CTRL/Y, `$FORCEX` and process deletion all reach a process this way. An
AST control block (ACB) names the process, the mode and the routine. An
AST is delivered when the process runs below IPL 2 in that mode or an
outer one, with that mode's ASTs enabled (`$SETAST`) and no AST routine
of that mode already running. Kernel mode ASTs, some of which run an
executive routine instead of a user's, do the executive's work in the
process's own context. `ASTLM` limits how many a process may have queued.

vaxpunk's are in `vms/exec/astdel.mar`
([ADR-0011](../../adr/0011-asts-on-the-kernel-stack.md)). The
executive keeps the ACBs and two bytes of the HWPCB, Alpha's ASTSR and
ASTEN, and the PAL requests the IPL 2 interrupt when they say an AST can
be delivered. The routine runs on top of the kernel stack, which holds
the interrupted registers, and `$ASTEXIT` puts it back. `$DELPRC`, I/O
completion ([ADR-0013](../../adr/0013-qio-irps-and-drivers.md)),
`$SETIMR`, `$FORCEX` and DCL's CTRL/Y
([ADR-0014](../../adr/0014-ctrlc-ctrly-asts.md)) are ASTs. Missing:
`ASTLM`, `IPL$_ASTDEL` around kernel code that holds pool (PRD-0003
tier 2 #13), and R0, R1, PC and PSL after the parameter (tier 3 #34).
