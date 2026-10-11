+++
title = "DIBOL"
summary = "DEC's business language for small systems, VAX DIBOL, now Synergy DBL"
aka = ["DIBOL", "VAX DIBOL", "DIBOL-83", "DBL", "Synergy DBL"]
kind = "layered"
intent = "aspiration"
research = "deferred"
status = "todo"

[[needs]]
on = "//rms/indexed"
why = "Its programs keep their data in ISAM files, RMS indexed files on VMS"

[[needs]]
on = "//exec/calling-standard"
why = "Calls system services and other languages by the standard"

[[needs]]
fuzzy = "A compiler that runs on vaxpunk, with decimal arithmetic, and its run-time library"
why = "The compiler and its run-time"
+++

DIBOL, the Digital Business Oriented Language, was DEC's language for
accounting and business programs on small machines, from the PDP-8
through the PDP-11 to VAX DIBOL on VMS, in the DIBOL-83 dialect:
records of fixed fields, decimal arithmetic, and ISAM files. Synergex's
DBL, a compatible language, carries it on as Synergy DBL, which still
runs on OpenVMS.

vaxpunk has none.
