+++
title = "HELP"
summary = "On-line help: topics and subtopics from help libraries"
aka = ["HELP", "HELP.EXE", "HELPLIB.HLB", "SYS$HELP", "HLP$LIBRARY", ".HLB", ".HLP", "LBR$OUTPUT_HELP"]
kind = "core"
intent = "goal"
research = "done"
refs = ["docs/adr/0017-command-tables-from-cld-with-vcdu.md", "vms/sysexe/help.mar", "vms/sysexe/help/describe.mar", "docs/prd/0003-multi-user-vms.md", "docs/prd/0006-tpu-and-eve.md", "docs/prd/0007-wasmrun.md"]

[[feature]]
id = "from-tables"
title = "HELP lists the verbs and describes their parameters, qualifiers and keywords from DCL$TABLES"
status = "done"
milestone = "m0"
note = "ADR-0017 decision 3"

[[feature]]
id = "utility-help"
title = "A utility's HELP from its own tables: TCPIP> HELP"
status = "done"
note = "help/describe.mar"

[[feature]]
id = "helplib"
title = "Text from SYS$HELP:HELPLIB.HLB: topics, subtopics, Topic? and Subtopic? prompts"
status = "todo"
needs = ["/rtl/lbr", "/host/vlib"]
note = "PRD-0003 tier 3 #27; describe.mar ponytail. vlib would build the library on the host"

[[feature]]
id = "user-libraries"
title = "HLP$LIBRARY, HLP$LIBRARY_1, ... and HELP/LIBRARY: a site's and a user's libraries"
status = "todo"
needs = ["#helplib"]

[[feature]]
id = "utility-libraries"
title = "Utilities' own help libraries: EVE's, WASM's, AUTHORIZE's"
status = "todo"
needs = ["#helplib"]
note = "PRD-0006 open question; PRD-0007 M7"

[[needs]]
on = "/dcl/cld"
why = "Describes the verbs from DCL$TABLES"

[[needs]]
on = "/dcl/interpreter"
why = "HELP is a verb whose image DCL runs"
+++

VMS's `HELP` reads help libraries, `.HLB` files that `LIBRARY/HELP`
builds from `.HLP` text: a topic, its text, and subtopics under it, any
number of levels deep. `HELP` alone lists the topics in
`SYS$HELP:HELPLIB.HLB`; `HELP COPY /LOG` goes down to one; at the end it
prompts `Topic?` or `COPY Subtopic?`. The logical names `HLP$LIBRARY`,
`HLP$LIBRARY_1` and on add a site's or a user's libraries, and
utilities (`MAIL`, `AUTHORIZE`, EVE) have their own, all read through
`LBR$OUTPUT_HELP`.

vaxpunk's `HELP.EXE` (`vms/sysexe/help.mar`) has no text. It describes
DCL's verbs from `DCL$TABLES`, the tables it is linked with: each verb's
parameters, qualifiers and keywords, and the syntaxes they switch to
([ADR-0017](../../adr/0017-command-tables-from-cld-with-vcdu.md)).
`help/describe.mar` does the same for `TCPIP>`. Text needs a help
library, `HELPLIB.HLB` (PRD-0003 tier 3 #27), which `LBR$OUTPUT_HELP`
in `LBRSHR.EXE` can now read and prompt through.
