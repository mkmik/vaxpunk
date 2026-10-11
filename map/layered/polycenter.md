+++
title = "POLYCENTER"
summary = "DEC's family of system management products: performance, scheduling, consoles"
aka = ["POLYCENTER", "DECps", "POLYCENTER Performance Solution", "POLYCENTER Scheduler", "DECscheduler", "POLYCENTER Console Manager", "POLYCENTER Security Compliance Manager", "DECinspect", "DECmcc"]
kind = "layered"
intent = "aspiration"
research = "deferred"
status = "todo"
refs = ["https://www.techmonitor.ai/?p=160946", "https://www.techmonitor.ai/?p=170903"]

[[needs]]
on = "//exec/information-services"
why = "Performance data comes from $GETJPI, $GETSYI and friends"

[[needs]]
on = "//utils/job-controller"
why = "The scheduler submits batch jobs by calendar and dependency"

[[needs]]
on = "//utils/opcom"
why = "Console and event managers watch operator messages"

[[needs]]
on = "//exec/security-audit"
why = "Security compliance and intrusion tools read audit events"
soft = true

[[needs]]
fuzzy = "Each product's own collector, database and DECwindows interface"
why = "A family, not one program; detail waits for one to be chosen"
+++

POLYCENTER was the name DEC gave its system and network management
products in the early 1990s. On VMS it covered performance collection
and advice (DECps, the POLYCENTER Performance Solution), job scheduling
across nodes (POLYCENTER Scheduler), console management (POLYCENTER
Console Manager), security checking and intrusion detection (from
DECinspect), and network management (DECmcc). `PRODUCT INSTALL`, PCSI,
began as the POLYCENTER Software Installation utility and is part of VMS
now ([//utils/pcsi](../utils/pcsi.md)). DEC sold most of the family to
Computer Associates in 1996, which merged it into Unicenter.

On vaxpunk it is an aspiration, kept as one component until one of its
pieces is chosen.
