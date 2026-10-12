+++
title = "DHCP client"
summary = "Gets the interface's address, mask, gateway and DNS server from DHCP"
aka = ["TCPIP$DHCP_CLIENT", "/DHCP", "SIOCSIFDHCP", "SIOCGIFDNS"]
kind = "core"
intent = "goal"
research = "done"
refs = ["docs/adr/0022-tcpip-utility-and-dhcp.md", "docs/adr/0026-name-service-and-nslookup.md", "pal/tcpip/include/lwipopts.h", "pal/tcpip/src/main.c"]

[[feature]]
id = "lease"
title = "Address, mask and gateway from DHCP, at START COMMUNICATION or /DHCP"
status = "done"
note = "ADR-0022"

[[feature]]
id = "dns-server"
title = "The DHCP server's DNS server (option 6) becomes TCPIP$BIND_SERVER000"
status = "done"
note = "ADR-0026, PR #144: names resolve behind a VPN"

[[feature]]
id = "arp-probe"
title = "An ARP probe of the offered address"
status = "todo"
note = "ADR-0022 ponytail: LWIP_DHCP_DOES_ACD_CHECK 0, which saves seconds at boot"

[[feature]]
id = "domain"
title = "The domain (option 15) and the second and third DNS servers"
status = "todo"
note = "ADR-0026 consequences"

[[feature]]
id = "renewals"
title = "Renewals and late answers reach VMS, and the lease shows"
status = "todo"
note = "ADR-0022 follow-up; a server after IFCONFIG's 10 seconds never reaches VMS"

[[feature]]
id = "native"
title = "A client process above the port, as TCP/IP Services' TCPIP$DHCP_CLIENT"
status = "todo"
note = "ADR-0022 alternatives: needs broadcasts from 0.0.0.0 and lease timers"

[[needs]]
on = "/platform/lwip"
why = "lwIP's DHCP client speaks the protocol, below the port"

[[needs]]
on = "/net/tcpip-qio"
why = "SIOCSIFDHCP asks for it and SIOCGIFDNS reads the DNS server"
+++

TCP/IP Services has a DHCP client process that configures each
interface marked `/DHCP`: it asks a DHCP server for an address, mask,
gateway, DNS servers and domain, keeps the lease and renews it.

In vaxpunk the client is lwIP's, in the TCP/IP component. `TCPIP SET
INTERFACE WE0 /DHCP`, or `START COMMUNICATION` with no saved address,
sends an `IFCONFIG` with the DHCP flag through the port, and the
component answers once the server has given an address, or after 10
seconds while it goes on asking
([ADR-0022](../../adr/0022-tcpip-utility-and-dhcp.md)). The
component also keeps the DNS server the server offers, which `TCPIP`
makes the system's first BIND server
([ADR-0026](../../adr/0026-name-service-and-nslookup.md)). QEMU's
user network and the web demo's LAN both have DHCP servers.
