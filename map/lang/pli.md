+++
title = "PL/I"
summary = "VAX PL/I and DEC PL/I, now Kednos PL/I for VAX and Alpha"
aka = ["VAX PL/I", "DEC PL/I", "Kednos PL/I", "DPLI$RTLSHR", "PLI$STARLET", ".PLI"]
kind = "layered"
intent = "aspiration"
research = "deferred"
status = "todo"
refs = ["https://kednos.com/pli/docs/AXP_RELEASE_NOTES/dpli_release_notes.html"]

[[needs]]
on = "//exec/calling-standard"
why = "Calls and is called by the standard"

[[needs]]
on = "//rms/core"
why = "PL/I's record and stream I/O are RMS files"

[[needs]]
on = "//lang/sdl"
why = "PLI$STARLET: the system's definitions in PL/I"
soft = true

[[needs]]
fuzzy = "A compiler that runs on vaxpunk, and the PL/I run-time library, DPLI$RTLSHR"
why = "The compiler and its run-time"
+++

DEC's PL/I was a full implementation of the ANSI G subset with VMS
extensions, from VAX PL/I to DEC PL/I on Alpha. DEC sold it to Kednos,
which still sells Kednos PL/I for VAX and Alpha; its run-time library,
`DPLI$RTLSHR`, once shipped with VMS and later came as a kit of its
own. `PLI$STARLET` holds the system services in PL/I. Kednos also
distributes the SDL extensions.

vaxpunk has none.
