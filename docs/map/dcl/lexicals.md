+++
title = "Lexical functions"
summary = "F$ functions: strings, files, logical names, processes, devices, time"
aka = ["F$", "F$GETJPI", "F$GETSYI", "F$GETDVI", "F$TRNLNM", "F$SEARCH", "F$PARSE", "F$FAO", "F$EDIT", "F$ELEMENT", "F$EXTRACT", "F$LOCATE", "F$MODE", "F$ENVIRONMENT", "F$CVTIME", "F$FILE_ATTRIBUTES", "F$CONTEXT", "F$GETQUI", "F$PRIVILEGE", "F$UNIQUE"]
kind = "core"
intent = "goal"
research = "done"
milestone = "m1"
refs = ["vms/sysexe/dcl/lexical.mar", "docs/prd/0003-multi-user-vms.md", "vms/sysmgr/dcltest.com", "vms/sysmgr/login.com"]

[[feature]]
id = "strings"
title = "F$LENGTH, F$EXTRACT, F$LOCATE, F$ELEMENT, F$EDIT, F$INTEGER, F$STRING, F$TYPE"
status = "done"
milestone = "m1"
note = "PRD-0003 step 8"

[[feature]]
id = "environment"
title = "F$MODE, F$ENVIRONMENT, F$VERIFY, F$TIME, F$MESSAGE"
status = "done"
milestone = "m1"
note = "PRD-0003 step 8"

[[feature]]
id = "names-files"
title = "F$TRNLNM, F$SEARCH, F$PARSE"
status = "done"
milestone = "m1"
note = "PRD-0003 step 8"

[[feature]]
id = "system-info"
title = "F$GETJPI, F$GETSYI, F$GETDVI, an item at a time"
status = "done"
milestone = "m1"
note = "PRD-0003 step 8"

[[feature]]
id = "fao"
title = "F$FAO: !AS, !UL, !SL, !ZL, !XL, !OL, !%S, !/, !_, widths"
status = "done"
milestone = "m1"

[[feature]]
id = "fao-directives"
title = "F$FAO's other directives: !%T, !%D, !#, !-, !+"
status = "todo"
milestone = "m1"
note = "PRD-0003 tier 1 #6"

[[feature]]
id = "search-streams"
title = "F$SEARCH with a context argument, several streams at once"
status = "todo"
needs = ["/rms/wildcards#streams"]
milestone = "m1"
note = "PRD-0003 tier 1 #6"

[[feature]]
id = "parse-related"
title = "F$PARSE's related specification and SYNTAX_ONLY"
status = "todo"
note = "lexical.mar ponytail"

[[feature]]
id = "user-node"
title = "F$USER, F$GETJPI USERNAME, node names"
status = "todo"
note = "lexical.mar ponytail: waits for LOGINOUT and the rest of PRD-0003"

[[feature]]
id = "time"
title = "F$CVTIME and F$DELTA_TIME"
status = "todo"
needs = ["/exec/time-conversion"]
note = "Needs $BINTIM (PRD-0003 tier 1 #7)"

[[feature]]
id = "file-attributes"
title = "F$FILE_ATTRIBUTES"
status = "todo"
needs = ["/rms/core"]

[[feature]]
id = "context"
title = "F$CONTEXT and F$PID: walk the processes that match"
status = "todo"
needs = ["/exec/information-services#process-scan"]

[[feature]]
id = "privileges"
title = "F$PRIVILEGE and F$SETPRV"
status = "todo"
needs = ["/exec/privileges"]

[[feature]]
id = "queues"
title = "F$GETQUI"
status = "todo"
needs = ["/utils/job-controller"]

[[feature]]
id = "identifiers"
title = "F$IDENTIFIER: names and values of rights identifiers"
status = "todo"
note = "Needs t8y:/exec/rights-database"

[[feature]]
id = "later"
title = "F$CVSI, F$CVUI, F$UNIQUE, F$FID_TO_NAME, F$LICENSE, F$MATCH_WILD, ..."
status = "todo"
note = "V7.3-2 added F$UNIQUE and F$DELTA_TIME; V8.2 F$FID_TO_NAME and F$LICENSE"

[[needs]]
on = "/dcl/expressions"
why = "Each argument is an expression, and the result is an operand"

[[needs]]
on = "/exec/information-services"
why = "F$GETJPI, F$GETSYI and F$GETDVI ask $GETJPI, $GETSYI, $GETDVI"

[[needs]]
on = "/exec/logical-names"
why = "F$TRNLNM is $TRNLNM"

[[needs]]
on = "/rms/wildcards"
why = "F$SEARCH and F$PARSE are $SEARCH and $PARSE"

[[needs]]
on = "/exec/messages"
why = "F$MESSAGE is $GETMSG"

[[needs]]
on = "/exec/timers"
why = "F$TIME is $GETTIM and $ASCTIM"
+++

Lexical functions are DCL's built-in functions, `F$name(arguments)`,
used anywhere an expression is. They work on strings (`F$EXTRACT`,
`F$ELEMENT`, `F$EDIT`), format with `$FAO`'s directives (`F$FAO`), and
ask the system: `F$GETJPI` about a process, `F$GETSYI` the system,
`F$GETDVI` a device, `F$TRNLNM` a logical name, `F$SEARCH` and
`F$PARSE` file specifications, `F$FILE_ATTRIBUTES` a file's header and
RMS attributes, `F$GETQUI` the queues, `F$CVTIME` dates. `F$MODE` says
whether the process is interactive, batch, network or other, which login
procedures test. VMS added functions in most releases; a V8.4 system has
about forty.

vaxpunk's are in `vms/sysexe/dcl/lexical.mar`, which PRD-0003 step 8
added with the list that PRD names; `LOGIN.COM` uses `F$MODE()` to skip
its terminal setup in batch. Each item-based function asks the service
for one item. `F$SEARCH` has one stream, `F$FAO` the common directives
(PRD-0003 tier 1 #6); the time, file attribute, queue and context
functions wait for the services under them.
