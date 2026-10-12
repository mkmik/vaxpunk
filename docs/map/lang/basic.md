+++
title = "BASIC"
summary = "VAX BASIC, DEC BASIC and VSI BASIC: a compiled BASIC with RMS record I/O"
aka = ["VAX BASIC", "DEC BASIC", "Compaq BASIC", "VSI BASIC", "BASIC-PLUS-2", ".BAS"]
kind = "layered"
intent = "aspiration"
research = "deferred"
status = "todo"
refs = ["https://docs.vmssoftware.com/"]

[[needs]]
on = "/rtl/basic-rtl"
why = "PRINT USING, MAP, record I/O and strings"

[[needs]]
on = "/exec/calling-standard"
why = "Calls system services and other languages by the standard"

[[needs]]
on = "/layered/cdd"
why = "%INCLUDE %FROM %CDD takes record definitions from CDD"
soft = true

[[needs]]
fuzzy = "A compiler that runs on vaxpunk, with floating point and packed decimal"
why = "The compiler itself"
+++

VAX BASIC grew out of the PDP-11's BASIC-PLUS-2: a compiled, structured
BASIC with dynamic strings, `MAP` and `RECORD` for record layouts, RMS
sequential, relative and indexed files, `PRINT USING`, and calls by the
standard, so a BASIC program could call any system service. Small
businesses and many commercial packages used it. It became DEC, Compaq
and HP BASIC on Alpha and Itanium, and VSI ships it for x86-64. It is a
layered product.

vaxpunk has none.
