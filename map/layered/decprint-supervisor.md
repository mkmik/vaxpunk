+++
title = "DECprint Supervisor"
summary = "DCPS: PostScript printer queues, translators and two-way printer links"
aka = ["DCPS", "DECprint Supervisor", "DCPS$SMB", "PostScript", "setup modules", "PrintServer", "CPAP"]
kind = "layered"
intent = "aspiration"
research = "deferred"
status = "todo"
refs = ["https://space.physics.uiowa.edu/pi/docs/vms83/sysman/dcpsv27/mguide/dcps027_mg_001.html", "https://space.physics.uiowa.edu/pi/docs/vms83/sysman/dcpsv27/uguide/dcps027_ug_contents.html"]

[[needs]]
on = "//utils/job-controller"
why = "Its printers are ordinary print queues"

[[needs]]
on = "//utils/print-symbiont"
why = "DCPS is a symbiont of its own, in place of the standard one"

[[needs]]
on = "//net/tcpip-qio"
why = "Raw TCP/IP and LPD links to network printers"

[[needs]]
on = "//drivers/terminal"
why = "Serial printers on terminal lines"
soft = true

[[needs]]
fuzzy = "Translators from text, ANSI, ReGIS, Tektronix and PCL to PostScript; setup modules in device control libraries"
why = "Printing anything on a PostScript printer"
+++

DECprint Supervisor, DCPS, is the software that drives PostScript
printers from VMS print queues. Its own symbiont talks to the printer
over a serial line, LAT, raw TCP/IP, LPD, AppleTalk or DECnet, and keeps
the link two-way where it can, so printer errors come back to the queue.
It translates text, ANSI, ReGIS, Tektronix and PCL into PostScript,
sends setup modules and forms from device control libraries, handles
page layout, sheet counts and paper trays, and reports page accounting.

On vaxpunk it is an aspiration. Milestone 1's printing is one symbiont
that writes to a terminal or a file, with no forms or device control
libraries ([PRD-0003](../../docs/prd/0003-multi-user-vms.md)).
