+++
title = "SMTP"
summary = "Internet mail: MAIL's SMTP% transport, the SMTP server and its queue"
aka = ["SMTP", "SMTP%", "TCPIP$SMTP", "MX"]
kind = "core"
intent = "aspiration"
research = "outline"
status = "todo"
refs = ["docs/prd/0002-networking.md"]

[[needs]]
on = "/utils/mail"
why = "VMS MAIL sends to SMTP%\"user@host\" through it"

[[needs]]
on = "/net/tcpip-qio"
why = "Connections on port 25"

[[needs]]
on = "/net/dns-resolver"
why = "MX records find a domain's mail server"

[[needs]]
on = "/utils/job-controller"
why = "Outgoing mail waits in a generic queue, as TCP/IP Services' does"
+++

TCP/IP Services connects VMS MAIL to Internet mail. Mail sent to
`SMTP%"user@host"` goes into a queue that the SMTP symbiont delivers,
looking up MX records; the SMTP server receives mail for local users and
puts it in their MAIL files. Mail between office VMS systems over TCP/IP
used this, as DECnet's used `NODE::USER`.

PRD-0002 lists mail between office VMS boxes as an application TCP/IP
should unblock, and PRD-0003 leaves mail to it, but which protocol
carries it is open. SMTP is TCP/IP Services' answer. Nothing is built.
