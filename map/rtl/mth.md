+++
title = "MTH$ routines"
summary = "The math library: SQRT, SIN, EXP, LOG and the rest, for every float type"
aka = ["MTH$", "MTH$SQRT", "DPML", "Portable Mathematics Library", "MATH$"]
kind = "core"
intent = "aspiration"
research = "deferred"
status = "todo"

[[needs]]
on = "//exec/calling-standard"
why = "F, D, G and IEEE floating arguments by the standard"

[[needs]]
fuzzy = "Floating point in vmacro and vasm, which have none yet (PRD-0007 Feasibility)"
why = "A math library needs FP instructions or soft-float"
+++

`MTH$` is VMS's math library: square roots, trigonometric, hyperbolic,
exponential and logarithmic functions, random numbers and complex
arithmetic, for VAX F, D, G and H floating and, on Alpha, IEEE S and T.
On Alpha, DEC's Portable Mathematics Library (DPML) is behind it and
behind the C library's `math.h`.

vaxpunk has no floating point yet: vmacro has no F, D or G instructions
and vasm encodes no FP arithmetic.
