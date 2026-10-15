+++
title = "Task-to-task over WebSocket"
summary = "DECnet's programming model, NODE::\"TASK=name\", SYS$NET and $QIO on NET:, over WebSocket"
aka = ["task-to-task", "NET:", "_NET:", "SYS$NET", "TASK=", "NETACP", "NETSERVER", "network objects", "NCB", "IO$M_INTERRUPT", "WebSocket", "RFC 6455"]
kind = "layered"
intent = "aspiration"
research = "outline"
refs = ["docs/prd/0002-networking.md", "https://www.rfc-editor.org/rfc/rfc6455", "https://www.rfc-editor.org/rfc/rfc1859"]

[[feature]]
id = "outgoing"
title = "Outgoing links: IO$_ACCESS on _NET: with a connect block, then one message per IO$_READVBLK and IO$_WRITEVBLK, and IO$_DEACCESS"
status = "todo"

[[feature]]
id = "incoming"
title = "Incoming links: NETACP checks the login, creates a network process, defines SYS$NET and runs the object's image or TASK=name.COM"
status = "todo"
needs = ["/exec/processes#create-delete"]

[[feature]]
id = "rms"
title = "RMS opens NODE::\"TASK=name\" as a sequential file, one message per $GET and $PUT, so DCL OPEN and every language's file I/O can use it"
status = "todo"
needs = ["/rms/remote#parse", "/dcl/file-io#open-close"]

[[feature]]
id = "non-transparent"
title = "Non-transparent links: an associated mailbox for connect and disconnect messages, declared names, reject, and 16 bytes of optional data"
status = "todo"
needs = ["/exec/mailboxes"]

[[feature]]
id = "interrupts"
title = "Interrupt messages, IO$M_INTERRUPT, as WebSocket text messages"
status = "todo"

[[feature]]
id = "proxies"
title = "Proxy logins from NETPROXY.DAT, for peers whose identity TLS or the tailnet has checked"
status = "todo"
needs = ["/utils/authorize#proxies"]

[[feature]]
id = "tls"
title = "Links over TLS, wss:"
status = "todo"
needs = ["/net/ssl#client", "/net/ssl#server"]

[[needs]]
on = "/net/tcpip-qio"
why = "Each link is one TCP connection"

[[needs]]
on = "/net/hosts-db"
why = "Node names, BOSTON as a host alias, in place of DECnet's node database"

[[needs]]
on = "/utils/loginout"
why = "Checks the login of each incoming link and starts its network process"

[[needs]]
fuzzy = "SHA-1 and base64 for the handshake's Sec-WebSocket-Accept; Mbed TLS has SHA-1 (ADR-0032)"
why = "Every WebSocket handshake needs them"
+++

DECnet itself stays dropped ([PRD-0002](../../prd/0002-networking.md)),
but what a DECnet program uses could come back on top of TCP/IP. A VMS
program never sees NSP, OSI transport, NSAPs or towers. It sees a file
name, `BOSTON"user pass"::"TASK=WHATTIME"`, that RMS opens as a
sequential file where each `$PUT` sends one message and each `$GET`
reads one. It sees `$QIO` on `_NET:` with `IO$_ACCESS`, `IO$M_ABORT`,
`IO$M_INTERRUPT` and `IO$_DEACCESS`, mailbox messages such as
`MSG$_CONNECT`, the logical names `SYS$NET`, `SYS$REM_NODE` and
`SYS$REM_ID`, and status codes such as `SS$_NOSUCHOBJ` and
`SS$_LINKDISCON`. If those stay exactly as on VMS, BASIC, Fortran,
Pascal, COBOL, C and DCL programs written for DECnet run unchanged,
whatever goes over the wire.

The wire would be WebSocket (RFC 6455), one connection per link. A
connect is an HTTP request, `GET /task/WHATTIME` with `Upgrade:
websocket`: the access control goes in an `Authorization` header and
the 16 bytes of optional data in another header, in base64. NETACP
holds the request open while it starts the server process, and answers
`101 Switching Protocols` when the program opens `SYS$NET`, or an error
when it rejects: 404 is `SS$_NOSUCHOBJ`, 401 `SS$_INVLOGIN`, 403
`SS$_REJECT`. Each record is one binary message, sent in fragments when
it is long, with the FIN bit on the last one, as the EOT bit marks it in
RFC 1859's DT TPDU. Interrupt messages are text messages holding their
bytes in base64, because WebSocket reserves its other frame types. A
disconnect is a Close frame, code 1000 with the optional data in its
reason, or a code from the 4000 range for an abort. Ping and Pong do
the job of NSP's inactivity timer.

WebSocket costs an HTTP exchange per link, done in NETACP, not in the
driver, and SHA-1 for the handshake. In return, every other system can
talk to a VMS task with ordinary tools: `websocat`, Python, Go, a Rust
crate in the boot test, or a browser page opening
`wss://boston/task/WHATTIME`. Links pass through TLS, proxies and
firewalls on port 443.

The pieces: a `NET:` driver next to the BG driver, which builds and
reads the frames (vaxpunk's BG driver already has the file name
`netdriver.mar`, so it needs another), and NETACP, a process that
listens, as the auxiliary server does for TCP/IP services
(t8y:/net/aux-server). FAL could then be one more network object,
number 17, carrying remote file access (t8y:/rms/remote), and MAIL
object 27.
