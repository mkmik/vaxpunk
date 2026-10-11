+++
title = "BIND name server"
summary = "A DNS server: BIND 9 serving zones, as a primary, secondary or cache"
aka = ["BIND", "named", "TCPIP$BIND", "DNS server", "load broker"]
kind = "core"
intent = "aspiration"
research = "deferred"
status = "todo"
refs = ["docs/research/openvms-dns.md"]

[[needs]]
on = "/net/tcpip-qio"
why = "UDP and TCP on port 53"

[[needs]]
on = "/net/dns-resolver"
why = "Shares the configuration and the tools"

[[needs]]
fuzzy = "Zone files and BIND 9's server"
why = "What a name server serves"
+++

TCP/IP Services includes ISC's BIND 9 server, so a VMS system can be a
primary or secondary name server for a zone, or a cache; the load broker
and metric server spread a cluster alias across its members by DNS. The
research report on DNS
([openvms-dns.md](../../research/openvms-dns.md)) defers it.

vaxpunk has only the resolver.
