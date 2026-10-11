+++
title = "Routing"
summary = "IP routes: the default gateway, the routing table, GATED and ROUTED"
aka = ["SET ROUTE", "SHOW ROUTE", "GATED", "ROUTED", "TCPIP$ROUTE.DAT"]
kind = "core"
intent = "goal"
research = "outline"
refs = ["docs/adr/0024-sockets-have-tcpip-services-qio-interface.md", "vms/exec/netdriver.mar", "vms/sysexe/tcpip.mar"]

[[feature]]
id = "default-route"
title = "One route, the default gateway"
status = "done"
note = "ADR-0024: SIOCADDRT, SIOCDELRT, SIOCGETRT"

[[feature]]
id = "table"
title = "A routing table: host and network routes, SHOW ROUTE"
status = "todo"
note = "PRD-0003 tier 3 #35; netdriver.mar ponytail"

[[feature]]
id = "dynamic"
title = "Dynamic routing with ROUTED (RIP) or GATED"
status = "todo"

[[needs]]
on = "//net/tcpip-qio"
why = "Routes are set and read with ioctls on a socket"

[[needs]]
on = "//platform/lwip"
why = "lwIP forwards by its interfaces and default gateway"
+++

TCP/IP Services keeps static routes, set with `TCPIP SET ROUTE` and
saved in `TCPIP$ROUTE.DAT`, and can run a routing daemon, ROUTED for RIP
or GATED for RIP, OSPF and the rest, so that a VMS system learns routes
or acts as a router. `SHOW ROUTE` lists the table.

vaxpunk has one route, the default gateway, which `SET ROUTE /DEFAULT
/GATEWAY` sets through `SIOCADDRT` and lwIP keeps. A routing table comes
when there is a second interface.
