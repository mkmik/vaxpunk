+++
title = "Terminal driver"
summary = "TTDRIVER: OPA0:, TNAn: and FTAn:, line editing, CTRL/C and CTRL/Y, characteristics"
aka = ["TTDRIVER", "ttdriver.mar", "OPA0:", "TNA", "TT", "TX", "terminal", "IO$_READPROMPT", "IO$_SETMODE", "SET TERMINAL", "SHOW TERMINAL", "$BRKTHRU", "CTRL/O", "CTRL/T", "XON", "XOFF", "class driver", "port driver"]
kind = "core"
intent = "goal"
research = "done"
milestone = "m0"
refs = ["docs/adr/0013-qio-irps-and-drivers.md", "docs/adr/0014-ctrlc-ctrly-asts.md", "docs/adr/0027-terminals-are-ucbs-and-telnet-is-in-the-driver.md", "docs/design/0002-executive-processes.md", "docs/prd/0003-multi-user-vms.md", "vms/exec/ttdriver.mar", "vms/sysexe/set.mar", "vms/sysexe/show.mar"]

[[feature]]
id = "reads-writes"
title = "Reads with prompts, echo, line editing and terminators; writes (OPA0:)"
status = "done"
milestone = "m0"
note = "ADR-0013"

[[feature]]
id = "ctrl-c-y"
title = "CTRL/C and CTRL/Y ASTs enabled with IO$_SETMODE"
status = "done"
milestone = "m0"
note = "ADR-0014"

[[feature]]
id = "read-modifiers"
title = "IO$M_NOFILTR, TIMED, ESCAPE, PURGE, TRMNOECHO, CVTLOW, TYPEAHDCNT; PASSALL, PASTHRU"
status = "done"
milestone = "m1"
note = "ADR-0027"

[[feature]]
id = "characteristics"
title = "IO$_SETMODE and IO$_SENSEMODE characteristics; SET TERMINAL, SHOW TERMINAL"
status = "done"
milestone = "m1"
note = "PRD-0003 step 3"

[[feature]]
id = "per-ucb"
title = "Each terminal a UCB of its own; TNAn: for TELNET, which the driver speaks"
status = "done"
milestone = "m1"
note = "PRD-0003 step 3, ADR-0027"

[[feature]]
id = "brkthru"
title = "$BRKTHRU: broadcast messages to terminals, for REPLY and the job controller"
status = "done"
milestone = "m1"
needs = ["/exec/system-services"]
note = "PRD-0003 tier 1 #3; done at once, no timeout, no SET BROADCAST classes (ttdriver.mar)"

[[feature]]
id = "hangup"
title = "A hangup deletes the terminal's process, not only its reads and writes"
status = "done"
milestone = "m1"
needs = ["/exec/processes"]
note = "PRD-0003 tier 1 #3"

[[feature]]
id = "more-units"
title = "More than 9 TELNET terminals: device names past 4 characters"
status = "todo"
milestone = "m1"
note = "PRD-0003 tier 1 #3; DESIGN-0002 I/O"

[[feature]]
id = "login-at-key"
title = "Unsolicited input on an idle terminal starts LOGINOUT, not a login at once"
status = "todo"
milestone = "m1"
note = "PRD-0003 tier 1 #2; exec.mar. VMS's driver tells the job controller, which starts t8y:/utils/loginout"

[[feature]]
id = "quotas"
title = "BIOLM, BYTLM charged for terminal reads and writes"
status = "todo"
milestone = "m1"
needs = ["/exec/quotas"]
note = "PRD-0003 tier 1 #4"

[[feature]]
id = "ctrl-o-t"
title = "CTRL/O, CTRL/T, XON and XOFF"
status = "todo"
note = "PRD-0003 tier 3 #29; ttdriver.mar ponytail"

[[feature]]
id = "formatted-writes"
title = "Formatted writes: wrap, tabs, form feeds, lowercase, IO$M_REFRESH redisplay"
status = "todo"
note = "PRD-0003 tier 3 #29; a write goes out as with IO$M_NOFORMAT"

[[feature]]
id = "itemlist-read"
title = "Item list reads (IO$_READVBLK!IO$M_EXTEND): initial string, recall from the caller"
status = "todo"
note = "What DCL's own recall buffer needs (PRD-0003 tier 3 #29)"

[[feature]]
id = "out-of-band"
title = "Out-of-band ASTs (IO$M_OUTBAND), unsolicited input, IO$M_READATTN"
status = "todo"

[[feature]]
id = "virtual-terminals"
title = "Virtual terminals (VTA): a session survives a disconnect, and reconnects"
status = "todo"

[[feature]]
id = "serial-lines"
title = "Serial lines and modem control: speed, parity, dial-up, hangup on carrier loss"
status = "todo"
note = "A third port; ADR-0027 would then give each UCB a port vector"

[[needs]]
on = "/exec/qio"
why = "Reads queue IRPs; completion is IOC$REQCOM"

[[needs]]
on = "/exec/asts"
why = "CTRL/C and CTRL/Y are ASTs it queues"

[[needs]]
on = "/exec/timers"
why = "A timed read's timer queue entry"

[[needs]]
on = "/exec/pool"
why = "IRPs and TNAn UCBs come from nonpaged pool"

[[needs]]
on = "/exec/ipl-sync"
why = "Editing and echo run at IPL$_SYNCH, input at IPL$_IOPOST"

[[needs]]
on = "/platform/console-uart"
why = "The console's characters come from the PAL's UART"

[[needs]]
on = "/drivers/network"
why = "A TNAn terminal sends and receives on its TCP connection (NET$TTIO)"
+++

VMS's terminal driver, `TTDRIVER`, is a class driver over port drivers
for each kind of line: the console, serial multiplexers, LAT, and
TCP/IP Services' TELNET terminals. Each terminal is a UCB with its
characteristics, type-ahead buffer and read in progress. A read echoes
and edits the line until a terminator, with a prompt, a timeout, an
escape sequence or no filtering, as screen programs need. CTRL/C,
CTRL/Y and other out-of-band characters deliver ASTs; CTRL/O discards
output, CTRL/T shows the process's state, XON and XOFF stop and start
it. `$BRKTHRU` writes broadcast messages through a read in progress and
redraws it. `SET TERMINAL` and `SHOW TERMINAL` set and show the
characteristics; virtual terminals keep a session across a disconnect.

vaxpunk's is `vms/exec/ttdriver.mar`. Each terminal is a UCB of its
own: the console, `OPA0:`, a TELNET terminal, `TNA1:` to `TNA9:`,
for each remote login, whose Telnet protocol the driver speaks over the
TCP connection ([ADR-0027](../../adr/0027-terminals-are-ucbs-and-telnet-is-in-the-driver.md)),
and a pseudo terminal, `FTA1:` to `FTA9:`, that a program drives with
the PTD$ routines (t8y:/drivers/pseudo-terminal, [ADR-0033](../../adr/0033-pseudo-terminals-are-a-port-of-the-terminal-driver.md)).
Reads have VMS's modifiers, terminator masks, line editing and a
16-line recall buffer; CTRL/C and CTRL/Y are ASTs
([ADR-0014](../../adr/0014-ctrlc-ctrly-asts.md)). `$BRKTHRU` writes
a message on a terminal, a user's or all of them, through a read in
progress, then writes the read's prompt and line again. What's left for
Milestone 1 is PRD-0003 tier 1 #3: more than 9 TELNET
terminals. CTRL/O, CTRL/T, XON/XOFF and
formatted writes are tier 3 #29.
