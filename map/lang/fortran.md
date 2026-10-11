+++
title = "Fortran"
summary = "VAX FORTRAN, DEC Fortran 90 and VSI Fortran: VMS's scientific language"
aka = ["FORTRAN", "VAX FORTRAN", "DEC Fortran", "Compaq Fortran", "HP Fortran", "VSI Fortran", "FORTRAN-77", "Fortran 90", ".FOR", ".F90"]
kind = "layered"
intent = "aspiration"
research = "deferred"
status = "todo"
refs = ["docs/design/0004-calling-standard.md", "https://docs.vmssoftware.com/"]

[[needs]]
on = "//rtl/fortran-rtl"
why = "Formatted I/O, logical units and run-time errors"

[[needs]]
on = "//rtl/mth"
why = "The intrinsic functions"

[[needs]]
on = "//exec/calling-standard"
why = "Arguments by reference, CHARACTER by descriptor (DESIGN-0004 C and Fortran)"

[[needs]]
on = "//lang/sdl"
why = "FORSYSDEF: the system's definitions in Fortran"
soft = true

[[needs]]
fuzzy = "A compiler that runs on vaxpunk, with F, D, G and IEEE floating point, and FP registers kept across exceptions"
why = "Floating point is the language's main work; the PAL's frame keeps no FP registers yet"
+++

FORTRAN was one of VMS's first languages: VAX FORTRAN, with DEC's
extensions to FORTRAN-77 (`STRUCTURE`, `RECORD`, `%VAL`, `%REF`,
`%DESCR`, `INCLUDE` from text libraries, `OPEN` keywords for RMS file
organizations), then DEC Fortran 90 on Alpha, renamed Compaq, HP and
VSI Fortran, now on x86-64. `FORSYSDEF.TLB` gives the system services
and their symbols in Fortran. Its calling conventions, arguments by
reference and strings by descriptor, are the ones the calling standard
was shaped around. It is a layered product.

vaxpunk has none. DESIGN-0004 says what a Fortran compiler must do to
conform; with no floating point in the toolchain or kept in exception
frames, the first need is there.
