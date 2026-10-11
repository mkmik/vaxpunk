+++
title = "Python"
summary = "CPython on OpenVMS: community ports, then VSI's Python 3.10 on x86-64"
aka = ["Python", "CPython", "Python 3", "python.wasm"]
kind = "layered"
intent = "aspiration"
research = "deferred"
status = "todo"
refs = ["https://docs.vmssoftware.com/"]

[[needs]]
on = "//lang/c"
why = "CPython is C"

[[needs]]
on = "//rtl/crtl"
why = "Files, processes and sockets through the C library"

[[needs]]
on = "//rtl/librtl"
why = "Extension modules are shareable images, found with LIB$FIND_IMAGE_SYMBOL"

[[needs]]
on = "//files/ods5"
why = "Module and package names need long, mixed-case file names"
soft = true

[[needs]]
on = "//rtl/pthreads"
why = "The threading module"
soft = true

[[needs]]
on = "//compat/wasmrun"
why = "CPython also builds for WASI, so a python.wasm could run with no C compiler on vaxpunk"
soft = true

[[needs]]
fuzzy = "Floating point in the C path, a far larger C run-time library, and more memory"
why = "What a CPython port asks of the system"
+++

Python has run on OpenVMS since the 1990s through community ports on
VAX and Alpha, built with DEC C and with modules for RMS, system
services and DCL symbols. VSI has a Python 3.10 with wheels for x86-64,
in field test. Like Perl and Java it lives on the C run-time library's
UNIX side.

vaxpunk has none.
