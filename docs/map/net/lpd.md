+++
title = "LPD"
summary = "LPR/LPD printing to and from UNIX systems, through VMS print queues"
aka = ["LPD", "LPR", "TCPIP$LPD", "TCPIP$LPRSETUP", "LPQ", "LPRM"]
kind = "core"
intent = "aspiration"
research = "deferred"
status = "todo"

[[needs]]
on = "/utils/job-controller"
why = "Print jobs go through VMS queues"

[[needs]]
on = "/utils/print-symbiont"
why = "The LPD client is a print symbiont"

[[needs]]
on = "/net/tcpip-qio"
why = "Port 515"
+++

TCP/IP Services' LPD client is a print symbiont that sends a VMS
queue's jobs to an LPD printer or UNIX host; its LPD server takes jobs
from UNIX systems and PCs and puts them in VMS queues.
`TCPIP$LPRSETUP` defines the printers. TELNETSYM covers printers that
take a TELNET connection instead.

vaxpunk has none, and no printing at all yet: PRD-0003 asks whether
`PRINT` is worth doing without a printer.
