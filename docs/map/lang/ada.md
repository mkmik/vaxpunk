+++
title = "Ada"
summary = "VAX Ada and DEC Ada, later GNAT: Ada with tasking on DECthreads"
aka = ["VAX Ada", "DEC Ada", "Compaq Ada", "GNAT", "GNAT Pro", "ACS", "Ada 83", "Ada 95"]
kind = "layered"
intent = "aspiration"
research = "deferred"
status = "todo"
refs = ["https://usenet.ada-lang.io/comp.lang.ada/rhh7rb$r05$1@dont-email.me/T/"]

[[needs]]
on = "/exec/calling-standard"
why = "Calls and is called by the standard"

[[needs]]
on = "/rtl/pthreads"
why = "Ada tasks run on DECthreads"

[[needs]]
on = "/lang/sdl"
why = "The STARLET package: the system's definitions in Ada"
soft = true

[[needs]]
fuzzy = "A compiler and its program library manager, and Ada's run-time library with tasking and exceptions as VMS conditions"
why = "The compiler, ACS and the run-time"
+++

DEC sold VAX Ada, an early validated Ada 83 compiler, and DEC
Ada on Alpha, with a program library manager, `ACS`, packages for the
system services and Motif bindings. Alsys had an Ada for VMS too.
AdaCore's GNAT Pro followed on Alpha and Itanium until AdaCore stopped
supporting OpenVMS in 2015; volunteers have kept a GNAT for Itanium,
and none ships for x86-64. Ada tasks map onto DECthreads, and Ada's
exceptions onto VMS conditions.

vaxpunk has none.
