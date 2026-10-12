+++
title = "Hosts database"
summary = "TCPIP$HOST.DAT, the local table of host names, aliases and addresses"
aka = ["TCPIP$HOST.DAT", "SET HOST", "SET NOHOST", "SHOW HOST", "HOST_ADDR", "HOST_LOCAL"]
kind = "core"
intent = "goal"
research = "done"
refs = ["docs/adr/0025-hosts-database.md", "docs/research/openvms-dns.md", "vms/sysexe/lib/inet.mar", "vms/sysexe/tcpip.mar"]

[[feature]]
id = "lookup"
title = "Programs look names up with HOST_ADDR, linked into every image"
status = "done"
note = "ADR-0025: PING, TELNET, SET HOST and COPY/HTTP take names"

[[feature]]
id = "layout"
title = "TCP/IP Services' 271-byte records and two keys"
status = "done"
note = "Settled on a real V5.7 system (research report)"

[[feature]]
id = "commands"
title = "SET HOST, SET NOHOST and SHOW HOST, with VMS's output and messages"
status = "done"
note = "ADR-0025"

[[feature]]
id = "messages"
title = "%TCPIP-I-DUPHOSTNAME, and -TCPIP-I-ALIAS for an alias"
status = "todo"
note = "ADR-0025 consequences: their codes aren't known yet"

[[feature]]
id = "logical"
title = "Found through the TCPIP$HOST logical name"
status = "todo"
note = "ADR-0025: RMS translates logical names only in the device part"

[[feature]]
id = "persistent"
title = "Kept across boots, in SYS$COMMON:[SYSEXE]"
status = "todo"
note = "ADR-0025 puts it on the ramdisk, filled by SYSTARTUP_VMS.COM"

[[feature]]
id = "networks"
title = "The networks database and CONVERT/VMS HOST from a UNIX hosts file"
status = "todo"

[[needs]]
on = "/rms/indexed"
why = "An indexed file, keyed by name and by address"

[[needs]]
on = "/drivers/ramdisk"
why = "Lives on MDA0:, made at each boot (ADR-0025)"
+++

TCP/IP Services looks a host name up first in its local hosts database,
`TCPIP$HOST.DAT`, an indexed RMS file of names, aliases and addresses,
then in DNS. `TCPIP SET HOST`, `SET NOHOST` and `SHOW HOST /LOCAL` keep
and show it; `CREATE HOST` makes it, seeded with `LOCALHOST`.

vaxpunk's has TCP/IP Services' record layout and keys, its commands
print what V5.7 prints, and the first command to find no file creates
it ([ADR-0025](../../adr/0025-hosts-database.md)). Programs search
it themselves with `HOST_ADDR` in `sysexe/lib/inet.mar`, then ask DNS;
on VMS gethostbyname asks the network ACP, which does the same. It lives
on the ramdisk, so the hosts a boot should know are `TCPIP SET HOST`
lines in `SYSTARTUP_VMS.COM`.
