+++
title = "Network ACP"
summary = "TCPIP$INETACP: serves IO$_ACPCONTROL lookups for gethostbyname"
aka = ["INETACP", "TCPIP$INETACP", "IO$_ACPCONTROL", "UCX$INETACP"]
kind = "core"
intent = "goal"
research = "outline"
refs = ["docs/research/openvms-dns.md", "docs/adr/0025-hosts-database.md", "docs/adr/0026-name-service-and-nslookup.md"]

[[feature]]
id = "acpcontrol"
title = "IO$_ACPCONTROL on BGA0: for GETHOSTBYNAME and GETHOSTBYADDR"
status = "todo"
note = "ADR-0025 and ADR-0026 follow-ups; the research report gives the codes and statuses"

[[feature]]
id = "budget"
title = "A short time budget per lookup, so one slow DNS answer doesn't block the rest"
status = "todo"
note = "Research report: the single queue was INETACP's bug"

[[needs]]
on = "//net/tcpip-qio"
why = "Lookups arrive as $QIO on the BG device"

[[needs]]
on = "//net/hosts-db"
why = "Searches the hosts database first, with HOST_LOCAL"

[[needs]]
on = "//net/dns-resolver"
why = "Then DNS, through the same resolver the programs link"

[[needs]]
on = "//exec/processes"
why = "A process serves the requests, since the driver can't read RMS files"
+++

On TCP/IP Services, gethostbyname and gethostbyaddr in `TCPIP$IPC_SHR`
don't resolve in the caller: they send an `IO$_ACPCONTROL` `$QIO` to a
BG device, and the network ACP process, `TCPIP$INETACP`, searches the
hosts database and then asks DNS through `TCPIP$ACCESS_SHR`. With the
stack stopped, gethostbyname fails even for a dotted address
([research report](../../docs/research/openvms-dns.md), *Experiments*).

vaxpunk has no ACP: programs look names up themselves with `HOST_ADDR`
([ADR-0025](../../docs/adr/0025-hosts-database.md)). Both ADRs name the
ACP as the next step, a process that serves `IO$_ACPCONTROL` on `BGA0:`
with the same library, so that C programs' gethostbyname works as it
does on VMS.
