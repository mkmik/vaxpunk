+++
title = "Languages"
band = "lang"
summary = "Compilers that run on VMS"
+++

The compilers VMS has had. DEC sold most of them as layered products:
C, C++, Fortran, Pascal, COBOL, BASIC, Ada, PL/I, BLISS, LISP, DIBOL,
DSM, and smaller ones such as RPG II, SCAN and APL. All of them follow
the calling standard, so a routine in one language calls a routine in
any other, and each brings its own run-time library (`map/rtl`) and its
own copy of the system's definitions, which DEC generated from one
source with SDL. The MACRO-32 compiler ships with the system. On Alpha
and Itanium most compilers shared one back end, GEM; on x86-64 VSI turns
GEM's intermediate language into LLVM's.

vaxpunk has two languages of its own, both compiled on the host today:
MACRO-32 (`vmacro`), the main kernel language, and BLISS-64 (`vbliss`)
for longer logic. The BLISS compiler is meant to compile itself on
vaxpunk, and a MACRO-32 compiler written in BLISS to follow. C comes
through a stock gcc and velf, only because SSL3 needed it. MACRO-64 is
dropped. Everything else here is an aspiration.
