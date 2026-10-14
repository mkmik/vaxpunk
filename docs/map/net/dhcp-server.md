+++
title = "DHCP server"
summary = "Gives addresses to other machines on the LAN, and answers BOOTP"
aka = ["TCPIP$DHCP_SERVER", "DHCP GUI"]
kind = "core"
intent = "aspiration"
research = "deferred"
status = "todo"

[[needs]]
on = "/net/tcpip-qio"
why = "Broadcast UDP on ports 67 and 68"

[[needs]]
on = "/net/bind-server"
why = "Dynamic DNS updates for the addresses it gives"
soft = true

[[needs]]
fuzzy = "A lease database and the server's configuration files"
why = "Remembers which machine has which address"

[[needs]]
on = "/net/aux-server"
why = "BOOTP and the DHCP server are started as services"
soft = true
+++

TCP/IP Services includes a DHCP server, which hands out addresses,
masks, gateways and other options to PCs and terminals on the LAN from
pools the system manager defines, keeps a database of leases, and also
answers BOOTP clients. It has its own configuration files and
management commands.

vaxpunk has none. On QEMU's network the host's DHCP server serves the
guest.
