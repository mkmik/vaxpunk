+++
title = "Auxiliary server"
summary = "VMS's inetd: starts a network process for each connection to a service"
aka = ["TCPIP$AUX", "inetd", "SET SERVICE", "ENABLE SERVICE", "TCPIP$SERVICE", "TCPIP$SERVICE.DAT", "SYS$NET", "TCPIP$C_AUXS", "network process"]
kind = "core"
intent = "aspiration"
research = "done"
refs = ["https://docs.vmssoftware.com/docs/vsi-tcpip-services-for-openvms-5-7-management.pdf", "https://docs.vmssoftware.com/docs/vsi-tcpip-services-for-openvms-management-command-reference.pdf", "https://docs.vmssoftware.com/docs/vsi-tcpip-services-for-openvms-sockets-api-and-system-services-programming.pdf", "https://docs.vmssoftware.com/docs/vsi-tcp-ip-services-for-openvms-tuning-and-troubleshooting.pdf", "https://github.com/endlesssoftware/netlib", "vms/exec/netdriver.mar"]

[[feature]]
id = "database"
title = "TCPIP$SERVICE.DAT, the permanent services database, which the logical name TCPIP$SERVICE names"
status = "todo"
note = "Management 1.5.1.2: a service's name, port, protocol, address, user name, process name, file, limit, flags and options. An indexed file whose layout no manual gives: settle it on a real V5.7 system, as ADR-0025 did TCPIP$HOST.DAT. getservbyname reads it too"

[[feature]]
id = "listen"
title = "Listens on each enabled service's port and address, TCP or UDP, and accepts its connections"
status = "todo"
needs = ["#database", "/net/tcpip-utility#services"]
note = "Started with the stack. The manuals don't name its process; SET COMMUNICATION's /ACCEPT, /REJECT and /PROXIES suggest it is part of INET_ACP. A change to a service reaches it only at DISABLE and ENABLE SERVICE"

[[feature]]
id = "network-process"
title = "A network process for each connection: the service's /USER_NAME, named from /PROCESS_NAME, running its /FILE procedure"
status = "todo"
needs = ["#listen", "/utils/loginout#network"]
note = "Management 1.5.1: 'dynamically creates a network process'. The account must be in SYSUAF.DAT, restricted, with network access only; SYLOGIN.COM and the account's LOGIN.COM run, and the log, such as [TCPIP$FTP]TCPIP$FTP_RUN.LOG, is new at each start. How names stay unique, and whether /FILE may be an image, aren't documented"

[[feature]]
id = "sys-net"
title = "SYS$NET names the connection's BG device, which the service takes over with IO$_SETMODE and af TCPIP$C_AUXS"
status = "todo"
needs = ["#network-process", "/net/tcpip-qio#auxs"]
note = "Sockets API 2.2.2 and socket(); C programs call socket(TCPIP$C_AUXS, SOCK_STREAM, 0) (t8y:/net/bsd-sockets#auxs). How SYS$NET reaches the new process's logical name table isn't documented"

[[feature]]
id = "limit"
title = "/LIMIT processes of a service at once; requests past it are rejected, not queued, as are more than 500 a minute"
status = "todo"
needs = ["#network-process"]
note = "Command Reference SET SERVICE; Tuning and Troubleshooting 2.2.3. SHOW SERVICE /FULL shows Active and Peak"

[[feature]]
id = "accept-reject"
title = "/ACCEPT and /REJECT by host and network, a service's and SET COMMUNICATION's; /REJECT=MESSAGE sent to the client"
status = "todo"
needs = ["#listen"]
note = "Up to 32 hosts and 16 networks; the message is up to 63 characters, framed by bytes 1 and 0 for RLOGIN, RSH and REXEC"

[[feature]]
id = "logging"
title = "/LOG_OPTIONS: accepts, rejects, logins, logouts, exits and errors to the service's log file and OPCOM"
status = "todo"
needs = ["#network-process", "/utils/opcom"]
note = "Events ACCEPT, ACTIVATE, ADDRESS, CONNECT, DEACTIVATE, ERROR, EXIT_CLEANUP, LOGIN, LOGOUT, MODIFY, REJECT. EXIT_CLEANUP logs a server that never took its socket. OPCOM's NETWORK class"

[[feature]]
id = "udp-listen"
title = "UDP services, flagged Listen: one process is handed the service's socket, as TFTP and BOOTP are, at /LIMIT=1"
status = "todo"
needs = ["#sys-net"]
note = "Sockets API: for a service with the LISTEN flag the auxiliary server 'creates a device socket' for the process. LISTEN is shown by SHOW SERVICE but isn't a SET SERVICE /FLAGS keyword"

[[feature]]
id = "socket-options"
title = "/SOCKET_OPTIONS and /PROTOCOL's options on a service's sockets: KEEPALIVE, buffer sizes, NODELAY, TTL"
status = "todo"
needs = ["#listen", "/net/tcpip-qio#options"]

[[feature]]
id = "proxy"
title = "/FLAGS=PROXY and APPLICATION_PROXY: the account from the proxy database or the client, for RSH, REXEC and LPD"
status = "todo"
needs = ["#network-process", "/net/tcpip-utility#proxies"]
note = "Startup says 'auxiliary server loaded with n proxy records'; /SEPARATOR parses REXEC's user name, password and command"

[[needs]]
on = "/net/tcpip-qio"
why = "Listens and accepts on each service's port"

[[needs]]
on = "/exec/processes"
why = "Creates a process for each connection"

[[needs]]
on = "/utils/authorize"
why = "Each service runs as a restricted account in SYSUAF.DAT"

[[needs]]
on = "/rms/indexed"
why = "The services database, TCPIP$SERVICE.DAT"
+++

TCP/IP Services' auxiliary server does what inetd does on UNIX, with
access control and event logging added. The system manager defines each
service with `TCPIP SET SERVICE`: its port, protocol and address, the
account it runs as, its process name, the command procedure that starts
it (`/FILE`), how many may run at once, and which hosts may connect.
`SET SERVICE` writes the permanent database, `TCPIP$SERVICE.DAT`;
`ENABLE SERVICE` and `DISABLE SERVICE` start and stop listening on the
running system, and `SET CONFIGURATION ENABLE SERVICE` names the
services enabled when the stack starts. For each connection the
auxiliary server accepts, it creates a network process logged in as the
service's account, a restricted one with network access only, which
runs the procedure, usually `TCPIP$service_RUN.COM` in
`SYS$SYSDEVICE:[TCPIP$service]`, with a fresh log file there. The
server program finds its connection through the logical name `SYS$NET`,
the BG device the auxiliary server accepted it on: it assigns a channel
to it and takes the socket over with `IO$_SETMODE` and the address
family `TCPIP$C_AUXS` (127), or in C with
`socket(TCPIP$C_AUXS, SOCK_STREAM, 0)`. DECnet's `SYS$NET` was a
connect block the object accepted or rejected itself; TCP/IP's is a
connection already made. FTP, REXEC, RSH, the SMTP receiver, POP, IMAP,
LPD, the NFS server, TFTP, BOOTP and user-written servers start this
way; TELNET and RLOGIN are entries too, but the stack's own terminals
serve them.

vaxpunk has none. Its one server, TELNETD, is a process of its own that
`START COMMUNICATION` creates and that listens on port 23 itself. Half
of the hand-over is there: a second process can `$ASSIGN` a `BGnn`
unit by name, and the unit stays until its last channel goes
(`netdriver.mar`). What's missing is the `TCPIP$C_AUXS` request
(`t8y:/net/tcpip-qio#auxs`), network logins
(`t8y:/utils/loginout#network`), and the database and commands.
