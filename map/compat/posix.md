+++
title = "POSIX for OpenVMS"
summary = "DEC's POSIX layered product, with a KornShell-based shell"
aka = ["POSIX for OpenVMS", "POSIX shell", "PSX$", "DECC$ POSIX features"]
kind = "layered"
intent = "aspiration"
research = "deferred"
status = "todo"

[[needs]]
on = "//rtl/crtl"
why = "POSIX's interfaces are C library calls"

[[needs]]
on = "//files/ods5"
why = "UNIX names and case"

[[needs]]
on = "//dcl/spawn"
why = "fork and exec map onto subprocesses"

[[needs]]
fuzzy = "fork semantics, signals, UNIX permissions and a POSIX file name space on Files-11"
why = "What POSIX.1 asks of the system"
+++

DEC released POSIX for OpenVMS in 1992, a layered product with the
POSIX interfaces, a POSIX file name space over Files-11, and a shell
based on the KornShell with the POSIX utilities. It was later retired
and replaced by GNV; the DEC C run-time library took on more of POSIX
itself, with `DECC$` feature logical names that choose UNIX behaviour.

vaxpunk has none.
