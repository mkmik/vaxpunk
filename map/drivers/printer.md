+++
title = "Printer"
summary = "LPA0: and printers on terminal ports: the device a print queue writes"
aka = ["LPA0:", "LP", "LPDRIVER", "line printer", "LRA0:", "parallel port", "SET PRINTER", "IO$M_NOFORMAT"]
kind = "core"
intent = "aspiration"
research = "outline"
status = "todo"
refs = ["docs/prd/0003-multi-user-vms.md"]

[[needs]]
on = "//exec/qio"
why = "Writes are $QIO requests"

[[needs]]
on = "//utils/print-symbiont"
why = "The symbiont writes a queue's jobs to it"
soft = true

[[needs]]
fuzzy = "A printer port: a parallel or serial port the PAL drives, or a host file"
why = "QEMU's virt machine has no printer"
+++

`LPA0:` is a line printer, driven by `LPDRIVER`: writes go to the
printer, formatted unless `IO$M_NOFORMAT`, with the page length,
lowercase and carriage control `SET PRINTER` sets. Most printers on
later VMS systems hung on a terminal port, a LAT port or the network
instead, and the print symbiont wrote to that device; Alpha workstations
had a parallel port as well. Nobody printed to the device directly: a
print queue owns it.

vaxpunk has no printer. PRD-0003's open question is whether `PRINT` is
worth doing without one; its step 10 has the symbiont write to a
terminal or a file, so the device is not needed for Milestone 1.
