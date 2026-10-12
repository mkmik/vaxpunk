+++
title = "Pascal"
summary = "VAX Pascal, DEC Pascal and VSI Pascal, with direct access to the system"
aka = ["VAX Pascal", "DEC Pascal", "Compaq Pascal", "HP Pascal", "VSI Pascal", "PASCAL", "STARLET.PEN", "[INHERIT]", ".PAS"]
kind = "layered"
intent = "aspiration"
research = "deferred"
status = "todo"
refs = ["https://docs.vmssoftware.com/"]

[[needs]]
on = "/rtl/pascal-rtl"
why = "File variables, text I/O and the heap"

[[needs]]
on = "/exec/calling-standard"
why = "Calls system services and other languages by the standard"

[[needs]]
on = "/lang/sdl"
why = "STARLET.PEN, the system's definitions as a Pascal environment"
soft = true

[[needs]]
fuzzy = "A compiler that runs on vaxpunk, with floating point and environment files"
why = "The compiler itself"
+++

VAX Pascal, later DEC, Compaq, HP and VSI Pascal, extended ISO Pascal
for systems work: separate compilation through environment files
(`[INHERIT('SYS$LIBRARY:STARLET')]` brings in every system service and
symbol), attributes for argument mechanisms and alignment, `VARYING`
strings, and `OPEN` on RMS files of any organization. Utilities and
whole applications were written in it. VSI ships it for x86-64. It is a
layered product.

vaxpunk has none.
