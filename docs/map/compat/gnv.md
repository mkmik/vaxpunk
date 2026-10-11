+++
title = "GNV (GNU on VMS)"
summary = "GNU's Not VMS: bash, coreutils, make and friends on the C library"
aka = ["GNV", "GNU's Not VMS", "bash", "coreutils"]
kind = "layered"
intent = "aspiration"
research = "deferred"
status = "todo"

[[needs]]
on = "/rtl/crtl#processes"
why = "bash runs commands with vfork and exec"

[[needs]]
on = "/rtl/crtl#unix-io"
why = "UNIX file names and file I/O"

[[needs]]
on = "/rtl/crtl#feature-logicals"
why = "The DECC$ feature logicals that make the library behave like UNIX's"

[[needs]]
on = "/files/ods5"
why = "UNIX names, case and symlinks"

[[needs]]
on = "/host/cross-gcc"
why = "There is no C compiler on vaxpunk to build it there"
+++

GNV, GNU's Not VMS, is the open-source project that replaced POSIX for
OpenVMS: ports of bash, the GNU coreutils, make, tar, gzip, grep, sed
and others, built on the DEC C run-time library with its UNIX feature
logicals set, so that UNIX build scripts and habits work on VMS. VSI has
continued it.

vaxpunk has none. It would need a far larger C run-time library than
the one SSL3 pulled in.
