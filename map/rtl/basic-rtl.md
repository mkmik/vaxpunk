+++
title = "BASIC run-time library"
summary = "VAX and DEC BASIC's library: PRINT USING, MAP, record I/O, strings"
aka = ["BAS$", "BASRTL", "DBASRTL"]
kind = "core"
intent = "aspiration"
research = "deferred"
status = "todo"

[[needs]]
on = "//exec/calling-standard"
why = "Called by compiled code by the standard"

[[needs]]
on = "//rms/core"
why = "BASIC's record I/O is RMS"

[[needs]]
on = "//rtl/str"
why = "Dynamic strings"

[[needs]]
on = "//rtl/librtl"
why = "Conditions, virtual memory and terminal I/O"
+++

The BASIC run-time library is what code compiled by VAX and DEC BASIC
calls: `PRINT`, `PRINT USING`, `INPUT` and `LINPUT`, file I/O with `MAP`
and record buffers on RMS, dynamic strings, decimal arithmetic, and
`ON ERROR` handling. On Alpha it is `DBASRTL.EXE`.
The library ships with VMS, so programs run without the compiler,
which is a layered product and may bring a newer one.

vaxpunk has no BASIC compiler and no library.
