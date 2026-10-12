+++
title = "Messages"
summary = "Condition values and their text: $GETMSG, $PUTMSG, $FAO, message files"
aka = ["$GETMSG", "$PUTMSG", "$FAO", "$FAOL", "condition value", "%FACILITY-S-IDENT", "$STSDEF", "SS$_", "message file", "SYS$MESSAGE", "SYSMSG.EXE", "SET MESSAGE", "message section", "facility", "STS$M_INHIB_MSG", "NOMSG"]
kind = "core"
intent = "goal"
research = "done"
milestone = "m0"
refs = ["docs/design/0002-executive-processes.md", "docs/prd/0003-multi-user-vms.md", "vms/exec/getmsg.mar", "vms/exec/sysunwind.mar", "vms/sysexe/dcl.mar", "vms/sysexe/dcl/lexical.mar", "crosstools/vtools/lib/starlet.mlb"]

[[feature]]
id = "condition-values"
title = "VMS's condition values: severity, message number, facility; SS$_ and RMS$_ codes"
status = "done"
milestone = "m0"
note = "$STSDEF, $SSDEF, $RMSDEF in starlet.mlb"

[[feature]]
id = "getmsg"
title = "$GETMSG from one table in the executive: SYSTEM's, RMS's and some of TCP/IP's"
status = "done"
milestone = "m0"
note = "getmsg.mar; NONAME-F-NOMSG for the rest"

[[feature]]
id = "putmsg"
title = "$PUTMSG writes %FAC-S-IDENT, text and -FAC lines, in the caller's mode"
status = "done"
milestone = "m0"
note = "sysunwind.mar; the catch-all handler and DCL use it"

[[feature]]
id = "fao-subset"
title = "FAO directives: !XB, !XW, !XL in $PUTMSG; !AS, !UL, !SL, !ZL, !XL ... in F$FAO"
status = "done"
note = "sysunwind.mar, dcl/lexical.mar; PRD-0003 tier 1 #6 notes F$FAO has some directives"

[[feature]]
id = "message-files"
title = "Message files: a facility's messages in its image or in a message file the image names"
status = "todo"
note = "PRD-0003 tier 3 #26; PRD-0007 M7's WASM-F- messages want them; compiled by t8y:/utils/message"

[[feature]]
id = "fao-service"
title = "$FAO and $FAOL as system services, with every directive"
status = "todo"

[[feature]]
id = "putmsg-full"
title = "$PUTMSG's action routine, facility name, SYS$ERROR as well as SYS$OUTPUT"
status = "todo"
note = "sysunwind.mar ponytail"

[[feature]]
id = "set-message"
title = "SET MESSAGE: which parts are shown, and a process's own message file"
status = "todo"

[[feature]]
id = "lib-signal"
title = "LIB$SIGNAL and LIB$STOP write through $PUTMSG with condition handling"
status = "todo"
note = "PRD-0003 tier 3 #26; t8y:/rtl/librtl's, with t8y:/exec/condition-handling"

[[needs]]
on = "/exec/system-services"
why = "$GETMSG is a service; $PUTMSG is in the vector"

[[needs]]
on = "/exec/qio"
why = "$PUTMSG writes with $QIOW"

[[needs]]
on = "/utils/message"
why = "Compiles a facility's .MSG file into messages $GETMSG finds"
soft = true
+++

Every VMS routine returns a condition value: a longword whose low
three bits are a severity (success, informational, warning, error,
severe), whose middle bits are a message number, and whose top bits
name a facility. `$GETMSG` turns one into text, `%RMS-E-FNF, file not
found`, from a message section: `SYS$MESSAGE:SYSMSG.EXE` for the
system's, the image's own, or a message file the process has added
with `SET MESSAGE`. `$PUTMSG` writes a list of conditions, the first
with `%` and the rest with `-`, filling each text's arguments with
`$FAO`, the formatting service (`!AS`, `!UL`, `!XL`, `!%T`...). A
facility's messages are written in a `.MSG` file and compiled by the
`MESSAGE` utility. DCL writes the message of each command's status, and
the catch-all handler that of a condition nobody handled.

vaxpunk's `$GETMSG` is `vms/exec/getmsg.mar`: one table in the
executive with the file system's, RMS's and some of TCP/IP Services'
messages, and `%NONAME-F-NOMSG` for any other. `$PUTMSG` is in
`sysunwind.mar`, runs in the caller's mode, and knows only the FAO
directives the executive's messages use; DCL's `F$FAO` has its own
handful. There are no message files, `SET MESSAGE` or `$FAO` service
yet: PRD-0003 tier 3 #26 adds message files when a utility brings its
own facility.
