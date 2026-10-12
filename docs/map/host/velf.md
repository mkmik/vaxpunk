+++
title = "velf (gcc objects to VMS objects)"
summary = "Converts gcc's AArch64 ELF objects into object modules that vlink links"
aka = ["velf", "velf.md", "GCC_FLAGS", "ELF to OBJ"]
kind = "tool"
intent = "induced"
research = "done"
refs = ["docs/adr/0032-c-run-time-library-and-ssl3-on-mbed-tls.md", "crosstools/vtools/crates/velf", "crosstools/vtools/docs/velf.md", "crosstools/vtools/examples/c"]

[[feature]]
id = "convert"
title = "Sections to psects, symbols to the GSD, AArch64 relocations to TIR stores"
status = "done"
note = "PR #141"

[[feature]]
id = "flags"
title = "The gcc flags that keep the calling standard: x18 fixed, no FP registers, frame pointers"
status = "done"
note = "velf.md The compiler's flags"

[[feature]]
id = "traceback"
title = "Debugger and traceback records, and descriptors so conditions reach C frames"
status = "todo"
note = "velf.md ponytail; DESIGN-0004 leaves it for another compiler's code"

[[needs]]
on = "/host/cross-gcc"
why = "Converts what it compiles"

[[needs]]
on = "/host/vms-obj"
why = "Writes object modules"

[[needs]]
on = "/host/vasm"
why = "Uses vasm's object writer and dates"
+++

Not a VMS piece. vaxpunk has no C compiler of its own, so a stock gcc
compiles C on the host and `velf` turns its ELF relocatable objects into
vaxpunk object modules, which `vlib` and `vlink` take as they take
MACRO-32's and BLISS-64's. Anything it can't express, such as a global
offset table or thread-local storage, is an error.

It exists because the C run-time library needed C, which it needed
because SSL3 is built on Mbed TLS
([ADR-0032](../../adr/0032-c-run-time-library-and-ssl3-on-mbed-tls.md)).
`examples/c` showed first that a BLISS-64 program can call a C library
this way.
