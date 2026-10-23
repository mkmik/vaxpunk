+++
title = "DCL interpreter"
summary = "DCL.EXE: reads commands, parses them, runs images in the process"
aka = ["DCL", "DCL.EXE", "CLI", "command interpreter", "DIGITAL Command Language", "CTRL/Y", "CONTINUE", "STOP", "RUN", "MCR", "SET PROMPT", "SET MESSAGE", "Extended DCL", "EDCL"]
kind = "core"
intent = "goal"
research = "done"
milestone = "m0"
refs = ["docs/adr/0006-cli-in-p1-runs-images-in-its-process.md", "docs/adr/0014-ctrlc-ctrly-asts.md", "docs/adr/0017-command-tables-from-cld-with-vcdu.md", "docs/adr/0029-loginout-and-sysuaf.md", "docs/design/0002-executive-processes.md", "vms/sysexe/dcl.mar", "vms/sysexe/dcl.mlb", "vms/sysexe/dcl/message.mar", "vms/sysexe/dcl/setshow.mar"]

[[feature]]
id = "images"
title = "Runs each command's image in the process: RUN, MCR, verbs with an IMAGE"
status = "done"
milestone = "m0"
note = "ADR-0006, ADR-0017"

[[feature]]
id = "ctrl-y"
title = "CTRL/Y stops the image and prompts on top of it; CONTINUE, STOP, EXIT"
status = "done"
milestone = "m0"
note = "ADR-0014: DCL's supervisor mode AST"

[[feature]]
id = "ctrl-y-privileges"
title = "At CTRL/Y, a privileged image's privileges put away until CONTINUE, as VMS's DCL$SAVE_PRIVS"
status = "todo"
needs = ["/exec/installed-images#fixed-list"]
note = "VMS V4.3 DCL IMAGECTRL.MAR: the process's permanent privileges at the prompt. vaxpunk keeps a known image's in PCB$Q_PRIV until rundown (process.mar), so PHONE's SYSNAM, WORLD and OPER, and SETPWD's SYSPRV, stay on at the CTRL/Y prompt; needed before more images get privileges"

[[feature]]
id = "messages"
title = "A failed status's message, from $GETMSG"
status = "done"
milestone = "m0"
note = "DESIGN-0002 The command interpreter; dcl/message.mar"

[[feature]]
id = "builtins"
title = "Done by DCL itself: SET and SHOW DEFAULT, SHOW DEVICES, DEFINE, DEASSIGN, SHOW LOGICAL"
status = "done"
milestone = "m0"
note = "dcl/setshow.mar, dcl/logical.mar; VMS's SHOW DEVICES and SHOW LOGICAL are SHOW.EXE's: t8y:/utils/set-show#show-logical"

[[feature]]
id = "logical-modes"
title = "ASSIGN, DEFINE/USER_MODE, /EXECUTIVE_MODE, /TRANSLATION_ATTRIBUTES, /JOB, /GROUP"
status = "todo"
needs = ["/exec/logical-names"]
note = "PRD-0003 tier 3 #25: access modes first; LNM$JOB and LNM$GROUP are tier 1"

[[feature]]
id = "show-devices-full"
title = "SHOW DEVICES/FULL"
status = "done"
note = "PRD-0003 tier 3 #24; setshow.mar ponytail: only what $GETDVI knows, no operation counts, owners or a disk's geometry"

[[feature]]
id = "from-uaf"
title = "Given to each login by LOGINOUT, as the UAF's DEFCLI says"
status = "done"
milestone = "m1"
note = "PRD-0003 step 4, ADR-0029: t8y:/utils/loginout activates it with EXE$LOGIN"

[[feature]]
id = "continuation"
title = "A hyphen at a line's end continues the command on the next"
status = "todo"
note = "dcl.mar ponytail"

[[feature]]
id = "set-prompt"
title = "SET PROMPT"
status = "done"
note = "dcl.mar ponytail: no /CARRIAGE_CONTROL"

[[feature]]
id = "ctrl-t"
title = "CTRL/T: a line of the process's state; SET CONTROL=T"
status = "todo"
needs = ["/drivers/terminal#ctrl-o-t", "/exec/information-services"]
note = "dcl.cld ponytail: no T; its line comes from $GETJPI (t8y:/exec/information-services)"

[[feature]]
id = "set-message"
title = "SET MESSAGE: which parts of a message show, and messages from message files"
status = "todo"
needs = ["/exec/messages"]
note = "PRD-0003 tier 3 #26; dcl.mar ponytail: texts for a few statuses only"

[[feature]]
id = "run-detach"
title = "RUN/DETACH and RUN/PROCESS_NAME: an image in a new process"
status = "done"
needs = ["/exec/processes"]
note = "dcl.mar ponytail: /PROCESS_NAME alone is detached too, not a subprocess (t8y:/dcl/spawn)"

[[feature]]
id = "run-process-qualifiers"
title = "RUN's other qualifiers for the new process: /INPUT, /OUTPUT, /ERROR, /PRIVILEGES, /UIC, quotas"
status = "todo"
needs = ["#run-detach"]
note = "dcl.mar ponytail: none of them yet, and an image that isn't there fails in the process, not with %RUN-F-PARSEFAIL"

[[feature]]
id = "limits"
title = "Longer commands and symbols, as Extended DCL: 4,096-byte lines, 8,192-byte values"
status = "todo"
note = "dcl.mlb: 255-byte lines and values, 64 symbols, 8 levels, 16 KB of procedures; VMS V7.3-2 raised its own"

[[needs]]
on = "/exec/image-activator"
why = "RUN and every verb with an IMAGE call $IMGACT"

[[needs]]
on = "/exec/asts"
why = "CTRL/Y is a supervisor mode AST it prompts inside"

[[needs]]
on = "/drivers/terminal"
why = "Reads commands with IO$_READPROMPT; enables CTRL/Y with IO$_SETMODE"

[[needs]]
on = "/dcl/cld"
why = "Parses every command with DCL$TABLES"

[[needs]]
on = "/exec/messages"
why = "$GETMSG gives a failed status's text"

[[needs]]
on = "/exec/logical-names"
why = "SYS$INPUT, SYS$DISK and SYS$LOGIN"

[[needs]]
on = "/exec/processes"
why = "STOP deletes a process; EXIT outside a procedure is $FORCEX"

[[needs]]
on = "/platform/access-mode-threads"
why = "Supervisor mode is a thread of the process (ADR-0005)"

[[needs]]
on = "/rms/core"
why = "SET DEFAULT and SHOW DEFAULT parse with $PARSE"
+++

DCL is VMS's command interpreter. LOGINOUT maps it into P1 of each
process it logs in, where it runs in supervisor mode for the life of the
process. It prompts with `$`, reads a line, puts symbols' values in it,
parses the command against its command tables, and either does the
command itself or runs an image in the same process, in user mode, and
gets control back when the image exits. CTRL/Y stops an image and gives
a `$` prompt on top of it; `CONTINUE` resumes the image and `STOP` ends
it. What DCL keeps, its symbols, procedures and open files, is in P1 and
outlives each image. OpenVMS V7.3-2 raised its limits to 4,096-byte
commands and 8,192-byte symbols ("Extended DCL").

vaxpunk's DCL is `DCL.EXE`, linked at `VA$C_CLI` in P1
([ADR-0006](../../adr/0006-cli-in-p1-runs-images-in-its-process.md)).
`vms/sysexe/dcl.mar` reads, parses and dispatches, and runs images; the
modules in `vms/sysexe/dcl/` do the rest. It parses with
`CLI$$DCL_PARSE` from `vms/sysexe/lib/cli.mar` (t8y:/rtl/cli), against
`DCL$TABLES` and the tables `SET COMMAND` made. CTRL/Y is its supervisor
mode AST ([ADR-0014](../../adr/0014-ctrlc-ctrly-asts.md)). Its limits
are fixed and small (`dcl.mlb`): 255-byte lines and values, 64 symbols, 8
procedure levels, and 16 KB for the procedures running at once.
