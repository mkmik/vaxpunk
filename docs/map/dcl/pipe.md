+++
title = "PIPE"
summary = "Several commands on one line: ;, &&, ||, pipelines, redirection"
aka = ["PIPE", "|", "&&", "||", "SYS$PIPE", "pipeline", "subshell", "PIPE/TRUSTED"]
kind = "core"
intent = "goal"
research = "done"
refs = ["vms/sysexe/dcl/pipe.mar", "vms/cld/dcl.cld", "https://h41379.www4.hpe.com/doc/84final/9996/9996pro_155.html"]

[[feature]]
id = "sequences"
title = "a ; b, a && b, a || b, from left to right"
status = "done"
note = "pipe.mar"

[[feature]]
id = "pipelines"
title = "a | b: each segment a subprocess, joined by mailboxes, read as SYS$PIPE"
status = "todo"
needs = ["/dcl/spawn", "/exec/mailboxes", "/rms/core"]
note = "pipe.mar ponytail: no SPAWN, no RMS on a mailbox (rms/core's devices feature)"

[[feature]]
id = "redirection"
title = "<, > and 2>: SYS$INPUT, SYS$OUTPUT and SYS$ERROR for one sequence"
status = "todo"
needs = ["/exec/logical-names"]
note = "VMS redefines them in supervisor mode, which needs logical names' access modes (PRD-0003 tier 3 #25)"

[[feature]]
id = "background"
title = "a & runs in a subprocess, without waiting"
status = "todo"
needs = ["#pipelines"]

[[feature]]
id = "subshells"
title = "( a ; b ) runs in a subprocess, waited for"
status = "todo"
needs = ["#pipelines"]

[[feature]]
id = "qualifiers"
title = "/TRUSTED, /NOLOGICAL_NAMES, /NOSYMBOLS, /PRIVILEGES"
status = "todo"
needs = ["#pipelines"]

[[needs]]
on = "/dcl/interpreter"
why = "Each command of the line is one DCL does"

[[needs]]
on = "/rms/core"
why = "A pipeline's programs read and write SYS$PIPE, a mailbox, with $GET and $PUT"
+++

`PIPE` runs several DCL commands from one line, in the UNIX style. `;`
runs one after another, `&&` the next only if the last succeeded, `||`
only if it failed. `a | b` is a pipeline: each segment runs in a
subprocess of its own, at the same time, and each reads the one before's
`SYS$OUTPUT` as its `SYS$INPUT`, also named `SYS$PIPE`, through a
mailbox. `( ... )` runs a subshell in a subprocess and waits for it;
`&` at the end runs the line in a subprocess without waiting. `<`, `>`
and `2>` redirect `SYS$INPUT`, `SYS$OUTPUT` and `SYS$ERROR` for one
sequence. The status of the PIPE is that of its last command.
`/NOLOGICAL_NAMES` and `/NOSYMBOLS` make the subprocesses faster to
create; `/TRUSTED` lets a captive account's procedure use it.

vaxpunk's `PIPE` (`vms/sysexe/dcl/pipe.mar`) does `;`, `&&` and `||`,
taking each command from the line before reading another. `|`, `&`,
`<` and `>` are refused: they need subprocesses, which wait for `SPAWN`
(t8y:/dcl/spawn), and RMS on a mailbox.
