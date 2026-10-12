+++
title = "Print symbiont"
summary = "PRTSMB: the process that formats a print queue's files and writes them out"
aka = ["PRTSMB", "PRINT", "symbiont", "PSM$", "SMB$", "LATSYM", "TELNETSYM", "flag page", "burst page", "SYSDEVCTL.TLB", "forms"]
kind = "core"
intent = "goal"
research = "outline"
refs = ["docs/prd/0003-multi-user-vms.md", "https://h41379.www4.hpe.com/doc/84final/4493/4493pro_059.html"]

[[feature]]
id = "basic"
title = "One symbiont that writes a print queue's files to a terminal or a file"
status = "todo"
milestone = "m1"
note = "PRD-0003 step 10, if its open question (is PRINT worth doing without a printer?) says yes"

[[feature]]
id = "pages"
title = "Flag, burst and trailer pages; /COPIES, /HEADER, /PAGES; a form's page length and width"
status = "todo"

[[feature]]
id = "devctl"
title = "Setup modules from device control libraries (SYSDEVCTL.TLB) per form"
status = "todo"
needs = ["/rtl/lbr"]

[[feature]]
id = "streams"
title = "One symbiont process serving up to 16 queues at once"
status = "todo"

[[feature]]
id = "printers"
title = "Real printers: LPA0:, LAT printers through LATSYM, network printers through TELNETSYM and LPD"
status = "todo"
note = "t8y:/drivers/printer, t8y:/net/lat, t8y:/net/lpd"

[[feature]]
id = "callable"
title = "PSM$ routines for a modified symbiont, SMB$ routines for one written from scratch"
status = "todo"

[[needs]]
on = "/utils/job-controller"
why = "The job controller starts it and sends it each job"

[[needs]]
on = "/exec/mailboxes"
why = "Job controller and symbiont talk through mailboxes"

[[needs]]
on = "/rms/sequential"
why = "Reads the files to print"

[[needs]]
on = "/drivers/terminal"
why = "With no printer, a terminal is the output device"

[[needs]]
on = "/drivers/printer"
why = "A line printer, once there is one"
soft = true
+++

A symbiont is the process that does a print queue's work. The job
controller starts it and sends it jobs; it opens each file, formats it
(page headers, flag, burst and trailer pages that say whose job it is,
the form's margins and page length, setup modules from a device control
library), and writes it to the queue's device. `PRTSMB.EXE` is VMS's
standard one; it can serve up to 16 queues at once. `LATSYM` prints to
LAT printers, and TCP/IP Services adds `TELNETSYM` and an LPD
symbiont. Sites changed the standard one with `PSM$` routines, which
replace a step such as the page header, or wrote their own with `SMB$`
routines.

vaxpunk has none. PRD-0003's step 10 asks for one symbiont that writes
to a terminal or a file, with no forms or device control libraries, if
its open question, whether `PRINT` is worth doing without a printer,
says yes. It comes after batch (step 9), which brings the job
controller and the queues it would serve.
