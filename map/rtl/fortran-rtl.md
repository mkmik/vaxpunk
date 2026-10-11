+++
title = "Fortran run-time library"
summary = "DEC Fortran's library: formatted I/O, logical units on RMS files, errors"
aka = ["FOR$", "FORRTL", "DEC$FORRTL"]
kind = "core"
intent = "aspiration"
research = "deferred"
status = "todo"
refs = ["docs/design/0004-calling-standard.md"]

[[needs]]
on = "//exec/calling-standard"
why = "Called by compiled code by the standard"

[[needs]]
on = "//rms/core"
why = "Logical units are RMS files"

[[needs]]
on = "//rtl/librtl"
why = "Conditions, virtual memory and terminal I/O"

[[needs]]
on = "//rtl/mth"
why = "The intrinsic math functions"
+++

The Fortran run-time library is what code compiled by DEC Fortran
calls: the I/O statements (`OPEN`, `READ`, `WRITE`, `FORMAT`) on logical
units, which it connects to RMS files or to `FOR0nn` logical names,
list-directed and namelist I/O, its error handling and messages, and
the intrinsics that aren't `MTH$`. On Alpha it is `DEC$FORRTL.EXE`; VAX
had `FORRTL`.
The library ships with VMS, so programs run without the compiler,
which is a layered product and may bring a newer one.

vaxpunk has no Fortran compiler and no library. DESIGN-0004 says how
Fortran's VMS conventions, arguments by reference and `CHARACTER` by
descriptor, sit on the calling standard.
