+++
title = "DNS resolver"
summary = "BIND resolver: TCPIP$BIND_* settings, lookups over UDP, nslookup"
aka = ["BIND resolver", "SET NAME_SERVICE", "TCPIP$BIND_SERVER000", "nslookup", "TCPIP$NSLOOKUP", "TCPIP$ACCESS_SHR"]
kind = "core"
intent = "goal"
research = "done"
refs = ["docs/adr/0026-name-service-and-nslookup.md", "docs/research/openvms-dns.md", "vms/sysexe/lib/dns.b64", "vms/sysexe/tcpip.b64", "vms/sysexe/tcpip$nslookup.b64"]

[[feature]]
id = "config"
title = "SET and SHOW NAME_SERVICE keep the TCPIP$BIND_* logical names"
status = "done"
note = "ADR-0026: process table first, /SYSTEM for the system's"

[[feature]]
id = "resolver"
title = "A resolver linked into every image: A and PTR over UDP, search rules, no cache"
status = "done"
note = "ADR-0026, dns.b64"

[[feature]]
id = "after-hosts"
title = "HOST_ADDR asks DNS for a name the hosts database hasn't"
status = "done"
note = "ADR-0026"

[[feature]]
id = "nslookup"
title = "nslookup, with BIND 9's output"
status = "done"
note = "ADR-0026: the one BIND tool VMS users remember"

[[feature]]
id = "show-host"
title = "SHOW HOST asks DNS too, in its BIND database layout"
status = "todo"
note = "ADR-0026 follow-up"

[[feature]]
id = "ivhost"
title = "%TELNET-E-IVHOST for a host neither the database nor DNS knows"
status = "todo"
note = "ADR-0025 and ADR-0026 follow-ups"

[[feature]]
id = "tcp"
title = "Truncated answers asked again over TCP; /TRANSPORT read"
status = "todo"
note = "ADR-0026 consequences"

[[feature]]
id = "aaaa"
title = "AAAA queries, for IPv6"
status = "todo"

[[feature]]
id = "resolv-conf"
title = "TCPIP$ETC:RESOLV.CONF and SET CONFIGURATION NAME_SERVICE"
status = "todo"
note = "Research report: defer; ADR-0026 has no permanent copy"

[[needs]]
on = "//net/tcpip-qio"
why = "Asks the servers over a UDP socket"

[[needs]]
on = "//exec/logical-names"
why = "Its settings are logical names, the process's before the system's"

[[needs]]
on = "//net/hosts-db"
why = "The hosts database answers first, and names the servers given by name"

[[needs]]
on = "//host/vbliss"
why = "The resolver, the commands and nslookup are BLISS-64"
+++

TCP/IP Services resolves names through BIND's resolver, configured by
logical names, `TCPIP$BIND_SERVER000` to `002`, `TCPIP$BIND_DOMAIN` and
the rest, which `TCPIP SET NAME_SERVICE` writes, process-wide or with
`/SYSTEM` system-wide. gethostbyname reaches it through the network
ACP, getaddrinfo through a BIND 9 resolver in the caller; neither
caches. nslookup, dig and host are the tools
([research report](../../docs/research/openvms-dns.md)).

vaxpunk keeps the same logical names and commands, and has a resolver
in BLISS-64, `sysexe/lib/dns.b64`, linked into every image, which sends
A and PTR queries over UDP with the V5 search rules and BIND 9's timing.
`HOST_ADDR` calls it after the hosts database, so every program that
takes a host takes a DNS name. `SYSTARTUP_VMS.COM` names `8.8.8.8`, and
the server DHCP offers replaces it. nslookup is
`SYS$SYSTEM:TCPIP$NSLOOKUP.EXE`
([ADR-0026](../../docs/adr/0026-name-service-and-nslookup.md)).
