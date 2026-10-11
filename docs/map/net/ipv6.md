+++
title = "IPv6"
summary = "IPv6 addresses, AF_INET6 sockets, AAAA lookups and IPv6 routing"
aka = ["IPv6", "AF_INET6", "TCPIP$IP6_SETUP", "IPNODES.DAT"]
kind = "core"
intent = "aspiration"
research = "outline"
status = "todo"
refs = ["docs/prd/0002-networking.md"]

[[needs]]
on = "/platform/lwip"
why = "lwIP has IPv6, off today (lwipopts.h)"

[[needs]]
on = "/net/tcpip-qio"
why = "AF_INET6 socket names and the IPv6 ioctls"

[[needs]]
on = "/net/dns-resolver"
why = "AAAA queries, which it doesn't send yet"

[[needs]]
on = "/net/tcpip-utility"
why = "Commands to set IPv6 addresses and routes"
+++

TCP/IP Services V5 has IPv6: `AF_INET6` sockets,
`getaddrinfo`, IPv6 addresses on interfaces, router discovery, the
IPv6 hosts file `IPNODES.DAT`, and `TCPIP$IP6_SETUP` to configure it.

PRD-0002 leaves IPv6 out ("lwIP may support it; not exposed yet"), and
the port's messages carry IPv4 addresses only.
