+++
title = "C compiler"
summary = "DEC C and its successors; here a stock gcc on the host, through velf"
aka = ["DEC C", "VAX C", "Compaq C", "HP C", "VSI C", "CC", "gcc", "/POINTER_SIZE", "/PREFIX_LIBRARY_ENTRIES"]
kind = "layered"
intent = "induced"
research = "outline"
refs = ["docs/adr/0032-c-run-time-library-and-ssl3-on-mbed-tls.md", "docs/design/0004-calling-standard.md", "crosstools/vtools/docs/velf.md", "crosstools/vtools/examples/c/README.md", "vms/c.rs"]

[[feature]]
id = "gcc"
title = "C compiled by a stock gcc on the host, its objects converted by velf"
status = "done"
needs = ["//host/cross-gcc", "//host/velf"]
note = "PR #141; ADR-0032"

[[feature]]
id = "arg-count"
title = "The argument count in x9 and sign-extended ints, so C calls VMS without jackets"
status = "todo"
note = "DESIGN-0004 C and Fortran; ADR-0032 weighed teaching gcc and chose jackets"

[[feature]]
id = "frames"
title = "Frame descriptors or unwind tables, so conditions reach C frames"
status = "todo"
needs = ["//exec/condition-handling"]
note = "DESIGN-0004: no condition may reach a C frame yet"

[[feature]]
id = "pointer-size"
title = "32-bit pointers in VMS structures, as DEC C's /POINTER_SIZE gives"
status = "todo"
note = "ADR-0032 decision 3: such addresses are integers; gcc's AArch64 ILP32 is deprecated"

[[feature]]
id = "floating-point"
title = "Floating point, once exception frames keep the FP registers"
status = "todo"
note = "velf.md: -mgeneral-regs-only, since the PAL's frame keeps x0-x30 and sp only"

[[feature]]
id = "native"
title = "A C compiler on vaxpunk: CC at the DCL prompt"
status = "todo"
note = "An aspiration; no PRD. It could share BLISS.EXE's IR and back end, as DEC's compilers shared GEM"

[[needs]]
on = "//host/cross-gcc"
why = "The compiler today"

[[needs]]
on = "//host/velf"
why = "Turns gcc's ELF objects into object modules"

[[needs]]
on = "//rtl/crtl"
why = "The library C programs link"

[[needs]]
on = "//exec/calling-standard"
why = "C conforms through gcc's flags and jackets (DESIGN-0004)"
+++

DEC's C compiler for VMS was VAX C, then DEC C, renamed Compaq C, HP C
and VSI C; VSI moved it to x86-64 by turning GEM's intermediate language
into LLVM's. It follows the VMS calling standard, can use 32- or 64-bit
pointers (`/POINTER_SIZE`), prefixes library calls with `decc$`, and
reads VMS's own headers, `starlet.h` and the rest. It is a layered
product; the C run-time library it calls is part of the system.

vaxpunk has no C compiler of its own and doesn't want one for itself:
C came in because SSL3 needed Mbed TLS
([ADR-0032](../../docs/adr/0032-c-run-time-library-and-ssl3-on-mbed-tls.md)).
A stock gcc compiles on the host, with flags that leave x18 alone, keep
frame pointers and stay off the FP registers, and velf turns its ELF
objects into object modules (PR #141). What gcc doesn't do of the
calling standard, the argument count in x9 and sign-extended ints,
jackets do, and no condition may reach a C frame
([DESIGN-0004](../../docs/design/0004-calling-standard.md)). A C
compiler running on vaxpunk is an aspiration.
