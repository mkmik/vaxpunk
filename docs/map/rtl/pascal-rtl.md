+++
title = "Pascal run-time library"
summary = "VAX and DEC Pascal's library: file variables, text I/O and the heap"
aka = ["PAS$", "PASRTL", "PAS$RTL"]
kind = "core"
intent = "aspiration"
research = "deferred"
status = "todo"

[[needs]]
on = "/exec/calling-standard"
why = "Called by compiled code by the standard"

[[needs]]
on = "/rms/core"
why = "Pascal's files are RMS files"

[[needs]]
on = "/rtl/librtl"
why = "Conditions, virtual memory and terminal I/O"
+++

The Pascal run-time library is what code compiled by VAX and DEC Pascal
calls: file variables and text I/O on RMS files, `OPEN` with VMS file
attributes, `NEW` and `DISPOSE` on the heap, and the run-time checks.
On Alpha it is `PAS$RTL.EXE`.
The library ships with VMS, so programs run without the compiler,
which is a layered product and may bring a newer one.

vaxpunk has no Pascal compiler and no library.
