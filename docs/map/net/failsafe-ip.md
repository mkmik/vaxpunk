+++
title = "failSAFE IP"
summary = "Moves an IP address to another interface, or node, when one fails"
aka = ["failSAFE IP", "TCPIP$FAILSAFE", "standby addresses"]
kind = "core"
intent = "aspiration"
research = "deferred"
status = "todo"

[[needs]]
on = "/net/tcpip-qio"
why = "Watches interfaces and moves addresses with ioctls"

[[needs]]
on = "/drivers/network"
why = "Needs a second network device to fail over to"

[[needs]]
on = "/exec/cluster"
why = "Across nodes, an address moves to another cluster member"
soft = true
+++

TCP/IP Services' failSAFE IP watches each interface and,
when one stops receiving, moves its IP addresses to a standby interface
on the same node or on another node of the cluster, so clients keep
their connections' address. It is configured with standby addresses in
`TCPIP$CONFIG` and runs as a process of its own.

vaxpunk has one interface and no clusters.
