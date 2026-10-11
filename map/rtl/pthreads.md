+++
title = "POSIX threads"
summary = "DECthreads: the POSIX threads library, PTHREAD$RTL, with CMA and tis_ calls"
aka = ["DECthreads", "PTHREAD$RTL", "CMA$RTL", "pthread", "tis_", "upcalls"]
kind = "core"
intent = "aspiration"
research = "outline"
status = "todo"

[[needs]]
on = "//exec/asts"
why = "Thread switching and timed waits are built on ASTs and timers"

[[needs]]
on = "//exec/timers"
why = "Time slices and timed condition waits"

[[needs]]
on = "//exec/kernel-threads"
why = "Threads on several CPUs, and a blocking call that doesn't stop the process"
soft = true

[[needs]]
on = "//rtl/crtl"
why = "The C library must be thread-safe under it"

[[needs]]
on = "//exec/condition-handling"
why = "Exceptions and cancellation are VMS conditions"
+++

DECthreads, later the POSIX Threads Library, is VMS's threads package,
`PTHREAD$RTL.EXE`: the POSIX 1003.1c `pthread_` interface, DEC's older
CMA interface, and the thread-independent services (`tis_`) that let a
library be thread-safe whether or not threads are loaded. On Alpha V7
and later it uses kernel threads and upcalls from the executive, so
threads of one process run on several CPUs and a thread that waits for
I/O doesn't stop the others. Java, Apache, the OSU web server and many
ported programs need it.

vaxpunk has none. PRD-0007 leaves threads out of WASMRUN's v1.
