+++
title = "Network device (BG)"
summary = "BG0: and its BGn units: the port driver to the TCP/IP component"
aka = ["BG0:", "BGn", "BGA0:", "netdriver.mar", "NET$FDT", "NET$DDT", "port driver", "TCPIP$DEVICE", "IO$_TTY_PORT", "NET$TTIO"]
kind = "core"
intent = "goal"
research = "done"
milestone = "m0"
refs = ["docs/adr/0016-tcpip-component-and-bga0.md", "docs/adr/0024-sockets-have-tcpip-services-qio-interface.md", "docs/adr/0027-terminals-are-ucbs-and-telnet-is-in-the-driver.md", "docs/adr/0044-the-io-database.md", "docs/design/0003-tcpip-port.md", "docs/prd/0002-networking.md", "vms/exec/netdriver.mar"]

[[feature]]
id = "port"
title = "Commands in the port's ring, a doorbell, responses by interrupt at IPL$_PORT"
status = "done"
milestone = "m0"
note = "PRD-0002, DESIGN-0003"

[[feature]]
id = "template"
title = "BG0:, a template a BGn unit is cloned from for each channel, whose IO$_SETMODE makes its socket"
status = "done"
milestone = "m0"
note = "ADR-0016, ADR-0044"

[[feature]]
id = "tcpip-qio"
title = "TCP/IP Services' functions, item lists and ioctls on TCP, UDP and ICMP sockets"
status = "done"
note = "ADR-0024"

[[feature]]
id = "tty-port"
title = "IO$_TTY_PORT: a TCP connection becomes a TNAn terminal"
status = "done"
milestone = "m1"
note = "ADR-0027"

[[feature]]
id = "device-scan"
title = "BG0: and its units seen by $DEVICE_SCAN and $GETDVI"
status = "done"
milestone = "m1"
note = "ADR-0044: units of the I/O database, DEV$M_NET and DEV$M_MBX, class 0, as TCP/IP Services'"

[[feature]]
id = "close-pool"
title = "A CLOSE that finds no pool still closes"
status = "todo"
milestone = "m1"
note = "PRD-0003 tier 2 #14; unit numbers skip units in use since ADR-0044's IOC$CLONE_UCB"

[[feature]]
id = "flags-options"
title = "Buffer lists, socket options, read and write flags, IO$M_NOW, attention ASTs"
status = "todo"
note = "PRD-0003 tier 3 #35; netdriver.mar ponytail"

[[feature]]
id = "clone-at-assign"
title = "A unit cloned at $ASSIGN, as TCP/IP Services does"
status = "done"
note = "ADR-0044: NETMBX, as VMS's $ASSIGN of a network device; NET$CLONEDUCB"

[[feature]]
id = "more-ports"
title = "More than one port: a second interface or stack"
status = "todo"
note = "netdriver.mar ponytail: port 0 only"

[[feature]]
id = "lan-driver"
title = "A LAN device of VMS's own (EWA0:, with LANCP) under a native stack"
status = "todo"
note = "PRD-0002 Replace-later path; would need t8y:/drivers/lan"

[[needs]]
on = "/exec/qio"
why = "Every socket call is a $QIO through NET$FDT"

[[needs]]
on = "/platform/lwip"
why = "The TCP/IP component answers the port's commands"

[[needs]]
on = "/platform/pal"
why = "The doorbell is MTPR PR$_DOORBELL; the reply is an interrupt the PAL delivers"

[[needs]]
on = "/exec/pool"
why = "Units and IRPs, a send's data copied into its IRP"

[[needs]]
on = "/exec/ipl-sync"
why = "Completion in the IPL$_NETPOST fork"

[[needs]]
on = "/drivers/lan"
why = "The VMS-style LAN driver, if the port gives way to it"
soft = true
+++

On VMS with TCP/IP Services, `BG0:` is the device programs reach the
network through, a template: a channel assigned to it goes to a `BGn`
unit of its own, which gets a socket with `IO$_SETMODE`. Under it
sits the stack, in the executive (`TCPIP$INTERNET_SERVICES`), and under
that the LAN drivers. MultiNet and TCPware had their own devices. The
socket functions themselves are t8y:/net/tcpip-qio.

vaxpunk's network driver is `vms/exec/netdriver.mar`, a port driver:
TCP/IP is lwIP in a component below the executive, which shares the
port's pages with it. Each request is a command in the port's ring and
a doorbell; the component answers in the response ring and interrupts
at `IPL$_PORT`, and a fork completes the IRPs
([DESIGN-0003](../../design/0003-tcpip-port.md),
[ADR-0016](../../adr/0016-tcpip-component-and-bga0.md)). `$ASSIGN`
clones a unit of `BG0:`'s for each channel, as TCP/IP Services'
([ADR-0044](../../adr/0044-the-io-database.md)). It also sends and receives for the terminal
driver, which drives `TNAn` terminals over TCP connections
([ADR-0027](../../adr/0027-terminals-are-ucbs-and-telnet-is-in-the-driver.md)).
The port is the seam PRD-0002 keeps: a native stack would take lwIP's
place without changing this driver.
