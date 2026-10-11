+++
title = "TCPIP utility"
summary = "TCPIP management commands: interfaces, routes, hosts, name service, PING"
aka = ["TCPIP", "TCPIP$UCP", "UCX", "TCPIP>", "START COMMUNICATION", "TCPIP$CONFIG"]
kind = "core"
intent = "goal"
research = "done"
refs = ["docs/adr/0022-tcpip-utility-and-dhcp.md", "docs/adr/0025-hosts-database.md", "docs/adr/0026-name-service-and-nslookup.md", "docs/design/0003-tcpip-port.md", "vms/sysexe/tcpip.mar", "vms/sysexe/tcpip.b64", "vms/sysexe/tcpip.cld"]

[[feature]]
id = "set-interface"
title = "SET INTERFACE WE0 /HOST /NETWORK_MASK, and SHOW INTERFACE"
status = "done"
milestone = "m0"
note = "DCL verbs in ADR-0016; TCP/IP Services' syntax in the TCPIP utility since ADR-0022"

[[feature]]
id = "utility"
title = "The TCPIP verb, its own command tables, the TCPIP> prompt and HELP"
status = "done"
note = "ADR-0022"

[[feature]]
id = "configuration"
title = "SET CONFIGURATION INTERFACE, SET ROUTE /PERMANENT, START COMMUNICATION"
status = "done"
note = "ADR-0022; saved on the ramdisk since ADR-0025"

[[feature]]
id = "dhcp"
title = "/DHCP on SET INTERFACE and SET CONFIGURATION INTERFACE"
status = "done"
needs = ["//net/dhcp-client"]
note = "ADR-0022"

[[feature]]
id = "set-route"
title = "SET ROUTE /DEFAULT /GATEWAY"
status = "done"
needs = ["//net/routing"]

[[feature]]
id = "ping"
title = "PING on a raw ICMP socket, until CTRL/C or /NUMBER_PACKETS"
status = "done"
note = "ADR-0024; ponytail: no minimum, average and maximum round trips"

[[feature]]
id = "show-full"
title = "SHOW INTERFACE /FULL, with the DHCP lease"
status = "todo"
note = "ADR-0022 follow-up"

[[feature]]
id = "persistent"
title = "A configuration that outlives a boot, TCPIP$CONFIGURATION.DAT"
status = "todo"
note = "ADR-0025 keeps TCPIP$CONFIG.DAT on the ramdisk, so each branch boots its own"

[[feature]]
id = "services"
title = "SET SERVICE, ENABLE SERVICE, SHOW SERVICE"
status = "todo"
note = "The auxiliary server's commands (//net/aux-server)"

[[feature]]
id = "show-sockets"
title = "SHOW DEVICE_SOCKET and the protocol counters (netstat)"
status = "todo"

[[feature]]
id = "startup"
title = "STOP COMMUNICATION, TCPIP$STARTUP.COM, TCPIP$SHUTDOWN.COM, TCPIP$CONFIG.COM"
status = "todo"

[[needs]]
on = "//net/tcpip-qio"
why = "Senses and sets the interface with ioctls on a UDP socket"

[[needs]]
on = "//net/hosts-db"
why = "SET HOST, SET NOHOST and SHOW HOST keep the hosts database"

[[needs]]
on = "//net/dns-resolver"
why = "SET NAME_SERVICE and SHOW NAME_SERVICE are its commands"

[[needs]]
on = "//rtl/cli"
why = "Parses its commands with CLI$DCL_PARSE and CLI$DISPATCH"

[[needs]]
on = "//rms/sequential"
why = "The saved configuration, TCPIP$CONFIG.DAT"

[[needs]]
on = "//drivers/ramdisk"
why = "Its files live on MDA0:, made afresh at each boot (ADR-0025)"
+++

`TCPIP` is TCP/IP Services' management utility, a DCL verb whose image
parses the rest of the line, or prompts `TCPIP>`, against its own
command tables, as MAIL and AUTHORIZE do. It sets interfaces (`WE0` is
the first Ethernet), routes, hosts, the name service and services, both
in the running system and, with `SET CONFIGURATION`, in the permanent
configuration that `START COMMUNICATION` applies at boot.
`TCPIP$CONFIG.COM` is the menu procedure that sets it all up the first
time.

vaxpunk's is `TCPIP.EXE`, MACRO-32 with the name service commands in
BLISS-64, and `sysexe/tcpip.cld`
([ADR-0022](../../docs/adr/0022-tcpip-utility-and-dhcp.md)). It has
`SET`/`SHOW INTERFACE`, `SET ROUTE /DEFAULT`, `SET CONFIGURATION
INTERFACE`, `START COMMUNICATION`, `PING`, `HELP`, the hosts commands
([ADR-0025](../../docs/adr/0025-hosts-database.md)) and the name service
commands ([ADR-0026](../../docs/adr/0026-name-service-and-nslookup.md)).
`SYSTARTUP_VMS.COM` runs `START COMMUNICATION`, which also starts
`TCPIP$TELNET`. The saved settings are on the ramdisk and start empty
at each boot, so a fixed address is a pair of commands in the startup
procedure.
