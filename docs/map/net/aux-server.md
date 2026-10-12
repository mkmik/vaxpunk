+++
title = "Auxiliary server"
summary = "VMS's inetd: starts a server process for each connection to a service"
aka = ["TCPIP$AUX", "inetd", "SET SERVICE", "TCPIP$SERVICE.DAT", "SYS$NET"]
kind = "core"
intent = "aspiration"
research = "outline"
status = "todo"

[[needs]]
on = "/net/tcpip-qio"
why = "Listens and accepts on each service's port"

[[needs]]
on = "/exec/processes"
why = "Creates a process for each connection"

[[needs]]
on = "/utils/authorize"
why = "Runs each service as the user SET SERVICE names"

[[needs]]
on = "/rms/indexed"
why = "The services database, TCPIP$SERVICE.DAT"
+++

TCP/IP Services' auxiliary server does what inetd does on UNIX. The
system manager defines each service with `TCPIP SET SERVICE`: its port,
protocol, image, user name and process limits, kept in the services
database, `TCPIP$SERVICE.DAT`. The auxiliary server listens on all
their ports, and for each connection creates a process running the
service's image under that user, which finds its socket through the
logical name `SYS$NET`, as DECnet objects did. FTP, the r-commands,
finger, POP and user-written servers start this way. `ENABLE SERVICE`
and `SHOW SERVICE` manage them.

vaxpunk has none. Its one server, TELNETD, is a process of its own that
`START COMMUNICATION` creates and that listens on port 23 itself.
