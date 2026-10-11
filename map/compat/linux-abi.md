+++
title = "Linux binaries"
summary = "An escape hatch: run native ARM64 Linux programs on vaxpunk, without seL4"
aka = ["Linux ABI", "Linux emulation", "ELF", "syscall translation"]
kind = "glue"
intent = "aspiration"
research = "deferred"
status = "todo"

[[needs]]
on = "//exec/processes"
why = "A Linux program runs as, or in, a VMS process"

[[needs]]
on = "//exec/memory"
why = "mmap, brk and a Linux address space layout"

[[needs]]
on = "//exec/image-activator"
why = "Loads ELF images as well as VMS ones"

[[needs]]
on = "//files/ods5"
why = "UNIX file names, case and symlinks"

[[needs]]
on = "//net/bsd-sockets"
why = "Linux's socket calls map onto TCP/IP Services' sockets"

[[needs]]
on = "//exec/kernel-threads"
why = "clone and futex for threaded programs"
soft = true

[[needs]]
fuzzy = "Linux system call translation from EL0 svc, signals, /proc and the rest of what Linux programs assume, or rebuilt programs with a patched toolchain"
why = "The ABI itself: translated per process, or programs rebuilt from source or post-processed"
+++

Marko expects porting modern software to vaxpunk to be hard, and wants
an escape hatch: running native ARM64 Linux binaries, a whole Linux
userland or single processes. It must survive seL4's removal, so it
can't be a Linux VM beside the executive, and vaxpunk itself usually
runs in a VM, which makes nesting awkward. He is open to translating
Linux system calls in the executive, to rebuilding Linux software from
source with toolchain changes, or to post-processing binaries.

Real VMS never had this; its UNIX story was POSIX for OpenVMS and GNV,
source-level layers on the C library. WASMRUN is the other route
vaxpunk has chosen for modern programs.
