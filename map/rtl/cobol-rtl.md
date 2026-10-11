+++
title = "COBOL run-time library"
summary = "DEC COBOL's library: ACCEPT, DISPLAY, file I/O, packed decimal"
aka = ["COB$", "COBRTL", "DEC$COBRTL"]
kind = "core"
intent = "aspiration"
research = "deferred"
status = "todo"

[[needs]]
on = "//exec/calling-standard"
why = "Called by compiled code by the standard"

[[needs]]
on = "//rms/indexed"
why = "Sequential, relative and indexed file I/O, with COBOL's file status codes"

[[needs]]
on = "//rtl/sor"
why = "The SORT and MERGE statements"

[[needs]]
on = "//rtl/librtl"
why = "Conditions, virtual memory and terminal I/O"
+++

The COBOL run-time library is what code compiled by DEC COBOL calls:
`ACCEPT` and `DISPLAY`, sequential, relative and indexed file I/O on
RMS with COBOL's file status codes, packed decimal arithmetic, and
`SORT` and `MERGE` through `SOR$`. On Alpha it is `DEC$COBRTL.EXE`.
The library ships with VMS, so programs run without the compiler,
which is a layered product and may bring a newer one.

vaxpunk has no COBOL compiler and no library.
