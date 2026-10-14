+++
title = "BSD sockets library"
summary = "socket, connect, send, recv and gethostbyname for C, on the $QIO interface"
aka = ["TCPIP$IPC_SHR", "UCX$IPC_SHR", "socket.h", "decc$socket_fd", "gethostbyname", "getaddrinfo"]
kind = "core"
intent = "goal"
research = "done"
refs = ["docs/prd/0002-networking.md", "docs/adr/0032-c-run-time-library-and-ssl3-on-mbed-tls.md", "vms/crtl/socket.c", "vms/crtl/include/socket.h"]

[[feature]]
id = "client"
title = "socket, connect, send, recv, close, decc$socket_fd, gethostbyname"
status = "done"
note = "ADR-0032; PRD-0002 step 7 started"

[[feature]]
id = "server"
title = "bind, listen, accept"
status = "todo"
note = "PRD-0002: left"

[[feature]]
id = "select"
title = "select, poll and non-blocking sockets"
status = "todo"
note = "ADR-0032 consequences: sockets block"

[[feature]]
id = "options"
title = "setsockopt, getsockopt, getsockname, getpeername, shutdown"
status = "todo"
needs = ["/net/tcpip-qio#options"]

[[feature]]
id = "auxs"
title = "socket(TCPIP$C_AUXS, SOCK_STREAM, 0): the connection the auxiliary server hands a service"
status = "todo"
needs = ["/net/tcpip-qio#auxs"]
note = "For t8y:/net/aux-server#sys-net; socket.c refuses an af other than AF_INET. netperf's netserver and CERN httpd call it"

[[feature]]
id = "acp-lookups"
title = "gethostbyname through the network ACP, as on VMS"
status = "todo"
needs = ["/net/inetacp#acpcontrol"]

[[feature]]
id = "getaddrinfo"
title = "getaddrinfo, getnameinfo and the res_* API"
status = "todo"

[[feature]]
id = "shareable"
title = "A shareable image, TCPIP$IPC_SHR, rather than linked into each program"
status = "todo"
note = "ADR-0032 decision 8"

[[needs]]
on = "/net/tcpip-qio"
why = "Each call is a $QIOW on a channel to TCPIP$DEVICE:"

[[needs]]
on = "/rtl/crtl"
why = "Lives in the C run-time library, whose descriptors name the channels"

[[needs]]
on = "/net/hosts-db"
why = "gethostbyname calls HOST_ADDR"
+++

On VMS the BSD sockets calls are a library over the `$QIO` interface,
`TCPIP$IPC_SHR` (`UCX$IPC_SHR` before V5), which the C run-time library
calls. A socket is a descriptor that names a channel to a BG device;
`decc$socket_fd` turns a channel made with `$ASSIGN` into one, and
`decc$get_sdc` goes back. PRD-0002 keeps it out of the executive: "QIO
is the native interface; sockets is only a userland library for ported
applications", and makes it step 7 of its work order.

vaxpunk's is in the C run-time library, `vms/crtl/socket.c`, started
because SSL3 needed a socket to read and write
([ADR-0032](../../adr/0032-c-run-time-library-and-ssl3-on-mbed-tls.md)):
`socket`, `connect`, `send`, `recv`, `close`, `decc$socket_fd` and
`gethostbyname`, each blocking. `bind`, `listen` and `accept`, `select`
and the options are left.
