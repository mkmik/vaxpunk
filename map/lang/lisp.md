+++
title = "LISP"
summary = "VAX LISP, DEC's Common Lisp for VMS and ULTRIX on the VAX"
aka = ["VAX LISP", "VAX Common Lisp", "Common Lisp", "Spice Lisp"]
kind = "layered"
intent = "aspiration"
research = "deferred"
status = "todo"
refs = ["https://en.wikipedia.org/wiki/VAX_Common_Lisp"]

[[needs]]
on = "//exec/memory"
why = "A large heap that grows"

[[needs]]
on = "//exec/calling-standard"
why = "Calls system services and routines in other languages"

[[needs]]
fuzzy = "A Common Lisp system: compiler, garbage collector and image save and restore"
why = "The whole implementation"
+++

VAX LISP was DEC's Common Lisp for VMS and ULTRIX on the VAX,
bootstrapped from CMU's Spice Lisp. It had an Emacs-like editor for
terminals and, on workstations, a DECwindows environment with an
editor, debugger and inspector, and it called VMS routines through the
calling standard. It was sold to Lucid while a fourth version was in
development. No Alpha version is recorded.

vaxpunk has none.
