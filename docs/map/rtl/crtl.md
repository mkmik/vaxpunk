+++
title = "C run-time library"
summary = "DEC C's library: decc$ names, stdio on records, sockets, VMS headers"
aka = ["DECC$CRTL", "DECC$SHR", "CRTL", "VAXC$", "decc$", "stdio", "DECC$CRTL.OLB"]
kind = "core"
intent = "induced"
research = "done"
refs = ["docs/adr/0032-c-run-time-library-and-ssl3-on-mbed-tls.md", "docs/design/0004-calling-standard.md", "vms/crtl", "vms/crtl/test/crtltest.c", "vms/c.rs"]

[[feature]]
id = "basics"
title = "Strings, memory, malloc on $EXPREG, strtol, time, getenv on logical names"
status = "done"
note = "ADR-0032, PR #143"

[[feature]]
id = "stdio"
title = "stdio on records: SYS$OUTPUT and SYS$INPUT, fopen for reading through RMS"
status = "done"
note = "ADR-0032"

[[feature]]
id = "headers"
title = "DEC C's and TCP/IP Services' headers: starlet.h, descrip.h, socket.h, ..."
status = "done"

[[feature]]
id = "jackets"
title = "Jackets for the system services that set x9 and sign-extend"
status = "done"
note = "decc$sys.m64; DESIGN-0004 C and Fortran"

[[feature]]
id = "exit-status"
title = "main's result and exit() as the image's condition value"
status = "done"

[[feature]]
id = "write-files"
title = "fopen for writing and appending, fseek, remove, rename"
status = "todo"
note = "ADR-0032 consequences: fopen reads only"

[[feature]]
id = "stderr"
title = "stderr to SYS$ERROR"
status = "todo"
note = "stdio.c ponytail"

[[feature]]
id = "float"
title = "printf and strtod with floating point, and math.h"
status = "todo"
note = "stdio.c ponytail: no %e, %f, %g"

[[feature]]
id = "timezone"
title = "Local time from SYS$TIMEZONE_DIFFERENTIAL"
status = "todo"
needs = ["/exec/timers#utc"]
note = "time.c ponytail; ADR-0032 follow-up"

[[feature]]
id = "argv0"
title = "argv[0] is the image's file name"
status = "todo"
note = "start.c ponytail"

[[feature]]
id = "efn-enf"
title = "EFN$C_ENF in $QIO, so the library doesn't wait on event flag 0"
status = "todo"
needs = ["/exec/event-flags#enf"]
note = "ADR-0032 follow-up"

[[feature]]
id = "unix-io"
title = "open, read, write on files; stat; directories; UNIX file names"
status = "todo"

[[feature]]
id = "processes"
title = "system, vfork and exec, popen, signals"
status = "todo"
needs = ["/dcl/spawn"]
note = "Over subprocesses, as DEC C's are (t8y:/dcl/spawn)"

[[feature]]
id = "shareable"
title = "DECC$SHR.EXE, one copy shared rather than linked into each program"
status = "todo"
needs = ["/rtl/librtl#data-vector"]
note = "ADR-0032 decision 8: needs a shareable image that calls another and exports data"

[[feature]]
id = "feature-logicals"
title = "DECC$ feature logical names (DECC$FILENAME_UNIX_REPORT, ...)"
status = "todo"

[[needs]]
on = "/host/cross-gcc"
why = "Compiled by the cross gcc, there being no C compiler on vaxpunk"

[[needs]]
on = "/host/velf"
why = "Turns gcc's ELF objects into object modules"

[[needs]]
on = "/rms/sequential"
why = "stdio reads and writes records through RMS"

[[needs]]
on = "/rtl/librtl"
why = "LIB$PUT_OUTPUT, LIB$GET_INPUT, LIB$GET_FOREIGN"

[[needs]]
on = "/exec/logical-names"
why = "getenv translates logical names"

[[needs]]
on = "/exec/memory"
why = "malloc grows P0 with $EXPREG"

[[needs]]
on = "/exec/messages"
why = "strerror(EVMSERR) is $GETMSG's text"

[[needs]]
on = "/host/vasm"
why = "The system service jackets are ARM64 assembler"
+++

DEC C's run-time library, `DECC$SHR.EXE` (`DECC$CRTL.OLB` to link
statically), is the ANSI C and much of the POSIX library on a
record-oriented system: stdio over RMS records, `exit` statuses as
condition values, sockets on TCP/IP Services, UNIX-style file names
mapped to VMS ones, `vfork` and `exec` over `$CREPRC`, and dozens of
`DECC$` feature logical names that choose UNIX or VMS behaviour. Its
routines are named `decc$name`, which the compiler maps calls to. VAX C
had its own, `VAXCRTL`.

vaxpunk's, in `vms/crtl`, exists because SSL3 needed C
([ADR-0032](../../adr/0032-c-run-time-library-and-ssl3-on-mbed-tls.md)):
DEC C's names and headers, stdio on `SYS$OUTPUT`, `SYS$INPUT` and RMS
files read a record at a time, `malloc` on `$EXPREG`, `getenv` on
logical names, sockets on `TCPIP$DEVICE:`, and ARM64 jackets for the
system services, since gcc sets no argument count. Addresses in VMS
structures are 32-bit integers, gcc's pointers being 64-bit. Each C
program links it from `DECC$CRTL.OLB`. Missing: writing files,
`stderr`, floating point in `printf`, time zones, processes and the
shareable image.
