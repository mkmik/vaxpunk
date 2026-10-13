+++
title = "TELNET and SET HOST"
summary = "Remote login: TELNETD's TNA terminals, and SET HOST and TELNET's client"
aka = ["TELNET", "SET HOST", "RTPAD", "TELNETD", "TCPIP$TELNET", "TNA", "TNDRIVER", "IO$_TTY_PORT", "TELNETSYM"]
kind = "core"
intent = "goal"
research = "done"
refs = ["docs/prd/0002-networking.md", "docs/adr/0016-tcpip-component-and-bga0.md", "docs/adr/0027-terminals-are-ucbs-and-telnet-is-in-the-driver.md", "docs/adr/0029-loginout-and-sysuaf.md", "vms/sysexe/telnetd.mar", "vms/sysexe/rtpad.mar", "vms/exec/ttdriver.mar", "vms/cld/telnet.cld"]

[[feature]]
id = "line-mode"
title = "SET HOST between two systems, a line at a time"
status = "done"
milestone = "m0"
note = "PRD-0002 step 8, ADR-0016"

[[feature]]
id = "tna-terminals"
title = "A remote login is a TELNET terminal, TNAn, that the terminal driver drives"
status = "done"
milestone = "m1"
note = "PRD-0003 step 3, ADR-0027: EDIT's keypad mode works over SET HOST"

[[feature]]
id = "character-mode"
title = "RTPAD goes a character at a time on WILL ECHO, with NAWS for the size"
status = "done"
note = "ADR-0027"

[[feature]]
id = "loginout"
title = "TELNETD runs LOGINOUT on the terminal: username and password"
status = "done"
milestone = "m1"
needs = ["/utils/loginout"]
note = "PRD-0003 step 4, ADR-0029"

[[feature]]
id = "telnet-port"
title = "TELNET host port, a connection to any port, a line at a time"
status = "done"

[[feature]]
id = "hangup"
title = "A hangup deletes the process, as VMS's does"
status = "done"
milestone = "m1"
note = "PRD-0003 tier 1 #3"

[[feature]]
id = "more-terminals"
title = "More than 9 TNA units"
status = "todo"
milestone = "m1"
note = "PRD-0003 tier 1 #3: a device name has 4 characters"

[[feature]]
id = "command-mode"
title = "TELNET> command mode at CTRL/]"
status = "done"
note = "PRD-0003 tier 3 #29; rtpad.mar ponytail"

[[feature]]
id = "telnetsym"
title = "TELNETSYM: print to a remote printer over TELNET"
status = "todo"

[[feature]]
id = "kerberos"
title = "Kerberos authentication of TELNET logins"
status = "todo"
note = "Needs t8y:/net/kerberos"

[[needs]]
on = "/net/tcpip-qio"
why = "TELNETD listens on port 23 and RTPAD connects to it"

[[needs]]
on = "/drivers/terminal"
why = "The terminal driver speaks Telnet on TNAn units (ADR-0027)"

[[needs]]
on = "/net/hosts-db"
why = "SET HOST and TELNET take a host's name (HOST_ADDR)"

[[needs]]
on = "/net/dns-resolver"
why = "Names the hosts database hasn't"
soft = true

[[needs]]
on = "/exec/processes"
why = "TELNETD creates a process for each login"

[[needs]]
on = "/net/kerberos"
why = "Kerberos authentication"
soft = true
+++

On VMS, `SET HOST node` was DECnet's remote login (CTERM, through
RTPAD), and `TELNET host` was TCP/IP Services'. The TELNET server,
inside the stack, gives each connection a TELNET terminal, `TNAn:`,
before `LOGINOUT` runs on it, so a remote user's programs see an
ordinary terminal. TELNETSYM is a print symbiont that sends a queue's
jobs to a printer over TELNET.

vaxpunk's `SET HOST` and `TELNET` are both `RTPAD.EXE` over TCP port 23.
`TELNETD.EXE`, process `TCPIP$TELNET`, accepts each connection, turns it
into a `TNAn:` terminal with `IO$_TTY_PORT`, which the executive's
terminal driver drives over the connection speaking Telnet, and starts
`LOGINOUT` on it
([ADR-0027](../../adr/0027-terminals-are-ucbs-and-telnet-is-in-the-driver.md),
[ADR-0029](../../adr/0029-loginout-and-sysuaf.md)). RTPAD starts a
line at a time and goes a character at a time when the other end offers
to echo, so EDIT and EVE work in a remote session; CTRL/] gives its
`TELNET>` prompt, where `CLOSE` ends the session and a blank line or
`CONNECT` goes back to it. A hangup, the connection closing, deletes the
process created on the terminal, as VMS's does. Left: more than 9
terminals.
