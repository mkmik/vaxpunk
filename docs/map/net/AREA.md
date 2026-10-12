+++
title = "Networking"
band = "net"
summary = "TCP/IP Services and the protocols on it"
+++

What TCP/IP Services for OpenVMS gives a system: sockets as `$QIO` on BG
devices, the `TCPIP` management utility, name resolution, remote login,
and the servers and clients of the Internet protocols, plus the networks
VMS had besides TCP/IP (DECnet, LAT, PATHWORKS). vaxpunk does TCP/IP
only: lwIP in a component below a port (`t8y:/platform/lwip`), the
executive's driver for `BGA0:` above it, and TCP/IP Services' own
interfaces and commands on top ([PRD-0002](../../prd/0002-networking.md),
[DESIGN-0003](../../design/0003-tcpip-port.md)). DECnet is dropped.
