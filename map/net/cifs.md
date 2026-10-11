+++
title = "CIFS file and print serving"
summary = "PC file and print serving: PATHWORKS, Advanced Server, CIFS (Samba)"
aka = ["PATHWORKS", "Advanced Server for OpenVMS", "CIFS", "Samba", "SMB", "PWIP", "LAN Manager"]
kind = "layered"
intent = "aspiration"
research = "deferred"
status = "todo"

[[needs]]
on = "//net/tcpip-qio"
why = "SMB over TCP, NetBIOS over UDP"

[[needs]]
on = "//files/ods5"
why = "Windows names and case need ODS-5"

[[needs]]
on = "//exec/acls"
why = "Windows permissions map to VMS ACLs"
soft = true

[[needs]]
on = "//exec/rights-database"
why = "Windows users and groups map to VMS identifiers"

[[needs]]
fuzzy = "The SMB protocol: a Samba port on a full C library, or an implementation of our own"
why = "What serves the shares"
+++

PATHWORKS for OpenVMS made a VMS system a LAN Manager file and print
server for DOS and Windows PCs; it became Advanced Server for OpenVMS,
which could act as a Windows NT domain controller, and on Itanium HP
replaced it with CIFS for OpenVMS, a port of Samba, which VSI still
ships. They map PC users to VMS accounts and share VMS directories and
print queues. The PWIP driver let PATHWORKS run over TCP/IP Services.

vaxpunk has none.
