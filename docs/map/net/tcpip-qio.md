+++
title = "Sockets as $QIO"
summary = "TCP/IP Services' socket interface: $QIO on BG devices, item lists, ioctls"
aka = ["BG", "BG0:", "BGA0:", "TCPIP$DEVICE", "$INETSYMDEF", "TCPIP$INETDEF", "UCX$INETDEF"]
kind = "core"
intent = "goal"
research = "done"
refs = ["docs/prd/0002-networking.md", "docs/adr/0016-tcpip-component-and-bga0.md", "docs/adr/0024-sockets-have-tcpip-services-qio-interface.md", "docs/design/0003-tcpip-port.md", "vms/exec/netdriver.mar", "vms/sysexe/tcptest.mar"]

[[feature]]
id = "tcp"
title = "TCP sockets: connect, bind, listen, accept, read, write, close"
status = "done"
milestone = "m0"
note = "PRD-0002 steps 4-6, ADR-0016"

[[feature]]
id = "names"
title = "TCP/IP Services' function codes, item lists and SOCKADDRIN names"
status = "done"
note = "ADR-0024: every symbol with TCPIP$INETDEF's name and value"

[[feature]]
id = "udp"
title = "UDP sockets, named and connected"
status = "done"
note = "ADR-0024"

[[feature]]
id = "raw-icmp"
title = "Raw ICMP sockets, for PING"
status = "done"
note = "ADR-0024"

[[feature]]
id = "ioctls"
title = "Interface and route ioctls: SIOCSIFADDR, SIOCGIFFLAGS, SIOCADDRT, ..."
status = "done"
note = "ADR-0024; SIOCSIFDHCP and SIOCGIFDNS are vaxpunk's"

[[feature]]
id = "shutdown"
title = "IO$_DEACCESS with IO$M_SHUTDOWN"
status = "done"

[[feature]]
id = "device-scan"
title = "BG0: and its units seen by $DEVICE_SCAN and $GETDVI"
status = "done"
milestone = "m1"
needs = ["/drivers/network#device-scan"]
note = "ADR-0044"

[[feature]]
id = "close-pool"
title = "A CLOSE that finds no pool still closes; unit numbers skip units in use"
status = "todo"
milestone = "m1"
note = "PRD-0003 tier 2 #14"

[[feature]]
id = "options"
title = "Socket options, read and write flags (MSG_PEEK), IO$M_NOW"
status = "todo"
note = "PRD-0003 tier 3 #35; DESIGN-0003 ponytail"

[[feature]]
id = "buffer-lists"
title = "Buffer lists, p5 and p6 of a read or write"
status = "todo"
note = "PRD-0003 tier 3 #35"

[[feature]]
id = "attention-asts"
title = "Read and write attention ASTs, and out-of-band data"
status = "todo"
note = "DESIGN-0003: the AST modifiers are accepted and do nothing"

[[feature]]
id = "clone-at-assign"
title = "A unit cloned at $ASSIGN, as TCP/IP Services does"
status = "todo"
note = "ADR-0024 alternatives; netdriver.mar ponytail"

[[feature]]
id = "errno-iosb"
title = "The UNIX error code in a failed request's IOSB"
status = "todo"
note = "ADR-0024 consequences"

[[feature]]
id = "interfaces"
title = "More than one interface (WE0 only today)"
status = "todo"
note = "PRD-0003 tier 3 #35"

[[feature]]
id = "auxs"
title = "IO$_SETMODE with af TCPIP$C_AUXS on SYS$NET: take over a socket another process accepted"
status = "todo"
note = "For the auxiliary server (t8y:/net/aux-server#sys-net). A second process can $ASSIGN a BGn unit already; SOCKMODE refuses a p1 on a unit with a socket (SS$_BADPARAM), starlet.mlb has no TCPIP$C_AUXS (127), and a unit has no owner or protection, so any process can assign any BGn"

[[needs]]
on = "/drivers/network"
why = "BG0: and its BGn units carry the requests (netdriver.mar)"

[[needs]]
on = "/exec/qio"
why = "Every socket call is a $QIO with an IOSB, an event flag and an AST"

[[needs]]
on = "/platform/lwip"
why = "The TCP/IP component below the port does TCP, UDP and ICMP"

[[needs]]
on = "/exec/privileges"
why = "Ports below 1024 and raw sockets need SYSPRV or BYPASS; ioctls OPER"

[[needs]]
on = "/exec/logical-names"
why = "TCPIP$DEVICE, a system logical name, names BG0:"
+++

On VMS with TCP/IP Services (UCX before V5), a program reaches the
network with `$QIO`. It assigns a channel to `BG0:` or `TCPIP$DEVICE`,
makes a socket with `IO$_SETMODE`, and from then on the channel is its
own BG unit: `IO$_ACCESS` connects, `IO$_READVBLK` and `IO$_WRITEVBLK`
move data, `IO$_DEACCESS` closes. Socket names go in item lists, and the
interface and routes are set with BSD's ioctls passed the same way. The
C sockets library, `TCPIP$IPC_SHR`, is built on this, and so are
MultiNet's and TCPware's compatible interfaces. The stack under it is a
port of BSD's, in the executive.

vaxpunk has the same interface ([ADR-0024](../../adr/0024-sockets-have-tcpip-services-qio-interface.md)):
TCP, UDP and raw ICMP sockets on units of `BG0:`, which `TCPIP$DEVICE` names, with
TCP/IP Services' function codes, item lists and symbol values in
`starlet.mlb`. The driver, `vms/exec/netdriver.mar`, turns each request
into a message on a port of shared pages, and lwIP, in a component of
its own below the port, does the protocols
([DESIGN-0003](../../design/0003-tcpip-port.md)). TELNETD, RTPAD,
TCPIP, nslookup, COPY/HTTP and the C library all use it. Missing, each
marked in the code: socket options and flags, buffer lists, attention
ASTs, out-of-band data, IPv6, a second interface, and the UNIX error
code in the IOSB.
