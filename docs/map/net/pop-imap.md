+++
title = "POP and IMAP"
summary = "POP3 and IMAP servers, so PC mail programs read VMS MAIL"
aka = ["POP", "POP3", "IMAP", "TCPIP$POP", "TCPIP$IMAP"]
kind = "core"
intent = "aspiration"
research = "deferred"
status = "todo"

[[needs]]
on = "/utils/mail"
why = "They read users' VMS MAIL files"

[[needs]]
on = "/net/tcpip-qio"
why = "Ports 110 and 143"

[[needs]]
on = "/utils/loginout"
why = "Users log in with their UAF passwords"

[[needs]]
on = "/net/ssl"
why = "POP and IMAP over TLS"
soft = true
+++

TCP/IP Services has a POP3 server and, in later releases, an IMAP
server, both
of which serve users' VMS MAIL folders to PC and UNIX mail clients,
with UAF passwords. With SMTP they made a VMS system a mail server for
an office of PCs.

vaxpunk has none.
