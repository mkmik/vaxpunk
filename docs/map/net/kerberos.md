+++
title = "Kerberos"
summary = "MIT Kerberos V5: tickets for TELNET, FTP, r-commands and login"
aka = ["Kerberos", "KINIT", "KDC", "GSSAPI", "Kerberos ACME"]
kind = "core"
intent = "aspiration"
research = "deferred"
status = "todo"

[[needs]]
on = "/net/tcpip-qio"
why = "Talks to the KDC over UDP and TCP"

[[needs]]
on = "/rtl/crtl"
why = "A port of MIT's Kerberos is C"

[[needs]]
on = "/net/ntp"
why = "Tickets need clocks within minutes of each other"
soft = true

[[needs]]
on = "/utils/loginout"
why = "Kerberos logins go through LOGINOUT's authentication"
+++

HP shipped MIT Kerberos V5 for OpenVMS in its Open Source Security
volumes: a client (`KINIT`, `KLIST`), a KDC, and the GSSAPI. TCP/IP
Services' TELNET, FTP and r-commands can authenticate with it, and a
Kerberos ACME agent lets LOGINOUT accept Kerberos credentials.

vaxpunk has none.
