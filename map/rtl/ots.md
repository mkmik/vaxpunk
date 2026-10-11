+++
title = "OTS$ routines"
summary = "Language-independent support: conversions, string moves, BLISS's helpers"
aka = ["OTS$", "OTS$CVT_L_TI", "OTS$MOVE3", "OTS$BLISS_STATIC_HANDLER", "LIBOTS"]
kind = "core"
intent = "aspiration"
research = "outline"
status = "todo"

[[needs]]
on = "//exec/calling-standard"
why = "Called by compiled code by the standard"
+++

`OTS$` holds the general-purpose routines compilers call from the code
they generate, which aren't any one language's: binary-to-text and
text-to-binary conversions (`OTS$CVT_L_TI`, `OTS$CVT_TI_L`), string
moves and fills, 64-bit and 128-bit arithmetic helpers, and condition
handlers some languages need. On Alpha they are in `LIBOTS.EXE`.

vaxpunk's compilers emit inline code instead; vbliss handles the one
case where Alpha BLISS calls `OTS$BLISS_STATIC_HANDLER` itself.
