+++
title = "REPLY"
summary = "Messages to users' terminals, and the operator's side of OPCOM"
aka = ["REPLY", "REPLY/ALL", "REPLY/ENABLE", "REPLY/TO", "$BRKTHRU", "broadcast", "SET BROADCAST", "REQUEST"]
kind = "core"
intent = "goal"
research = "outline"
refs = ["docs/prd/0003-multi-user-vms.md", "docs/adr/0027-terminals-are-ucbs-and-telnet-is-in-the-driver.md", "vms/exec/ttdriver.mar", "https://typewritten.org/Manual/DEC/VMS/5.4/helplib.hlb/REPLY.html"]

[[feature]]
id = "broadcast"
title = "REPLY/ALL, /USERNAME, /TERMINAL: a message on users' terminals, with $BRKTHRU"
status = "todo"
milestone = "m1"
needs = ["/drivers/terminal#brkthru", "/exec/privileges"]
note = "PRD-0003 Terminals and tier 1 #3: no $BRKTHRU yet (ttdriver.mar)"

[[feature]]
id = "urgent"
title = "/URGENT, /SHUTDOWN and /BELL; SET BROADCAST to choose what a terminal takes"
status = "todo"
needs = ["#broadcast"]

[[feature]]
id = "operator"
title = "REPLY/ENABLE and /DISABLE: a terminal becomes an operator's, for some classes"
status = "todo"
note = "Needs OPCOM running (t8y:/utils/opcom)"

[[feature]]
id = "requests"
title = "REPLY/TO, /ABORT and /PENDING answer users' REQUEST/REPLY"
status = "todo"
needs = ["#operator"]

[[feature]]
id = "log"
title = "REPLY/LOG: close the operator log and open a new one"
status = "todo"
needs = ["#operator"]

[[feature]]
id = "tapes"
title = "/BLANK_TAPE and /INITIALIZE_TAPE for MOUNT's requests"
status = "todo"
needs = ["#requests"]
note = "Tapes: t8y:/drivers/tape"

[[needs]]
on = "/drivers/terminal#brkthru"
why = "$BRKTHRU writes on terminals between a program's reads"

[[needs]]
on = "/exec/privileges"
why = "OPER to send to other users and to be an operator"

[[needs]]
on = "/utils/opcom"
why = "Its operator functions talk to OPCOM, which must be running"
soft = true
+++

`REPLY` does two jobs. For anyone with `OPER` it sends a message to the
terminals of all users, some users or some terminals (`REPLY/ALL
"System going down in 10 minutes"`), through `$BRKTHRU`, which writes
it between a program's reads and puts the screen back; `/URGENT` and
`/SHUTDOWN` mark it, `/BELL` rings, and `SET BROADCAST` lets a user
choose which kinds a terminal takes. `SHUTDOWN.COM` warns users this
way. For operators it is OPCOM's front end: `REPLY/ENABLE=TAPES` makes
a terminal an operator terminal for a class of messages, `REPLY/TO`
and `/ABORT` answer a user's `REQUEST/REPLY` or a `MOUNT` waiting for a
tape, and `REPLY/LOG` starts a new `OPERATOR.LOG`.

vaxpunk has neither. PRD-0003 names `$BRKTHRU`, "for `REPLY` and the
job controller's messages", among Milestone 1's terminal requirements,
and its tier 1 #3 says it is still missing from `ttdriver.mar`. The
operator side waits for OPCOM, which is not planned.
