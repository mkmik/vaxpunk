+++
title = "C++ compiler"
summary = "DEC C++, then HP's, then VSI's clang-based C++ on x86-64"
aka = ["C++", "CXX", "DEC C++", "Compaq C++", "HP C++", "VSI C++", "CXXLINK", "libc++"]
kind = "layered"
intent = "aspiration"
research = "deferred"
status = "todo"
refs = ["https://llvm.org/devmtg/2017-10/slides/Reagan-Porting%20OpenVMS%20Using%20LLVM.pdf", "https://docs.vmssoftware.com/"]

[[needs]]
on = "/lang/c"
why = "Shares the C compiler's headers, conventions and object path"

[[needs]]
on = "/rtl/crtl"
why = "The C++ library sits on the C run-time library"

[[needs]]
on = "/exec/condition-handling"
why = "C++ exceptions unwind through VMS frames and meet VMS conditions"

[[needs]]
fuzzy = "A C++ standard library and its ABI support, and unwind information for every frame"
why = "What a C++ program links and throws through"
+++

DEC C++ (`CXX`) came to VMS in the early 1990s, with its own class
library and `CXXLINK`, which links and resolves templates. On Itanium,
HP's C++ was built on Intel's and EDG's front end rather than GEM; on
x86-64, VSI's C++ is based on clang, with libc++, extended for VMS's
32- and 64-bit pointers, command line, listings and messages. It is a
layered product.

vaxpunk has none, and nothing pulls one in. A C++ program would meet
the C path's limits (no argument count, no condition through a gcc
frame, no floating point), and exceptions would need unwinding through
VMS frames, which the calling standard's frame descriptors could give.
