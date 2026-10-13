+++
title = "Network device (BG)"
summary = "BGA0: and its BGnn units: the port driver to the TCP/IP component"
aka = ["BGA0:", "BGnn", "BG0:", "netdriver.mar", "NET$FDT", "port driver", "TCPIP$DEVICE", "IO$_TTY_PORT", "NET$TTIO"]
kind = "core"
intent = "goal"
research = "done"
milestone = "m0"
refs = ["docs/adr/0016-tcpip-component-and-bga0.md", "docs/adr/0024-sockets-have-tcpip-services-qio-interface.md", "docs/adr/0027-terminals-are-ucbs-and-telnet-is-in-the-driver.md", "docs/design/0003-tcpip-port.md", "docs/prd/0002-networking.md", "vms/exec/netdriver.mar"]

[[feature]]
id = "port"
title = "Commands in the port's ring, a doorbell, responses by interrupt at IPL$_PORT"
status = "done"
milestone = "m0"
note = "PRD-0002, DESIGN-0003"

[[feature]]
id = "template"
title = "BGA0:, a template whose IO$_SETMODE clones a BGnn unit per socket"
status = "done"
milestone = "m0"
note = "ADR-0016"

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
title = "BGA0: and its units seen by $DEVICE_SCAN and $GETDVI"
status = "todo"
milestone = "m1"
note = "PRD-0003 tier 1 #9"

[[feature]]
id = "close-pool"
title = "A CLOSE that finds no pool still closes; unit numbers skip units in use"
status = "todo"
milestone = "m1"
note = "PRD-0003 tier 2 #14"

[[feature]]
id = "flags-options"
title = "Buffer lists, socket options, read and write flags, IO$M_NOW, attention ASTs"
status = "todo"
note = "PRD-0003 tier 3 #35; netdriver.mar ponytail"

[[feature]]
id = "clone-at-assign"
title = "A unit cloned at $ASSIGN, as TCP/IP Services does"
status = "todo"
note = "netdriver.mar ponytail"

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
network through: a channel assigned to it gets a socket with
`IO$_SETMODE`, and from then on is a `BGnn` unit of its own. Under it
sits the stack, in the executive (`TCPIP$INTERNET_SERVICES`), and under
that the LAN drivers. MultiNet and TCPware had their own devices. The
socket functions themselves are t8y:/net/tcpip-qio.

vaxpunk's network driver is `vms/exec/netdriver.mar`, a port driver:
TCP/IP is lwIP in a component below the executive, which shares the
port's pages with it. Each request is a command in the port's ring and
a doorbell; the component answers in the response ring and interrupts
at `IPL$_PORT`, and a fork completes the IRPs
([DESIGN-0003](../../design/0003-tcpip-port.md),
[ADR-0016](../../adr/0016-tcpip-component-and-bga0.md)). `BGA0:`
clones a unit per socket. It also sends and receives for the terminal
driver, which drives `TNAn` terminals over TCP connections
([ADR-0027](../../adr/0027-terminals-are-ucbs-and-telnet-is-in-the-driver.md)).
The port is the seam PRD-0002 keeps: a native stack would take lwIP's
place without changing this driver.
