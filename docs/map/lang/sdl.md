+++
title = "SDL"
summary = "Structure Definition Language: one source for the system's definitions"
aka = ["SDL", "Structure Definition Language", "SDLEXT", ".SDL", "STARLET.SDL", "OpenSDL"]
kind = "layered"
intent = "aspiration"
research = "deferred"
status = "todo"
refs = ["https://kednos.com/kednos/Integration/SDL", "https://docs.vmssoftware.com/", "crosstools/vtools/crates/vdefs", "docs/prd/0004-bliss64-compiler.md"]

[[needs]]
fuzzy = "SDL's language, and a back end for each language: MACRO-32, BLISS, C, Fortran, Pascal, PL/I, Ada"
why = "The compiler and what it writes"
+++

SDL, the Structure Definition Language, is how DEC kept one definition
of each structure, constant and entry point and gave it to every
language: an `.SDL` source describes `$PCBDEF` or `SYS$QIO` once, and
the SDL compiler writes the MACRO-32 macros, BLISS require files, C
headers, Fortran, Pascal, PL/I and Ada declarations. The system's own
`STARLET` and `LIB` libraries in each language came out of it. DEC used
it internally and put it on the Freeware CD; Kednos sells extensions,
and VSI documents its own.

vaxpunk has no SDL. Its single source is the macro libraries,
`lib.mlb` and `starlet.mlb`: `vdefs` makes the BLISS require files from
their `$xxxDEF` macros, and PRD-0004 says it could write C headers too.
The C run-time library's headers (`vms/crtl/include`) are written by
hand.
