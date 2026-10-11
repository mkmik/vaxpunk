+++
title = "rlogin, rsh, rexec"
summary = "BSD's remote commands: rlogin, rsh, rexec, rcp, and the RMT tape server"
aka = ["RLOGIN", "RSH", "REXEC", "RCP", "RMT", "COPY/RCP"]
kind = "core"
intent = "aspiration"
research = "deferred"
status = "todo"

[[needs]]
on = "/net/tcpip-qio"
why = "Privileged ports for the r-commands' trust"

[[needs]]
on = "/drivers/pseudo-terminal"
why = "rlogin's server gives each login a terminal"

[[needs]]
on = "/utils/loginout"
why = "Logins and proxies checked against the UAF"
+++

TCP/IP Services has BSD's remote commands: rlogin, rsh and rexec, as
clients and servers, `RCP` and `COPY/RCP`, and the RMT server, which
lets UNIX systems use VMS tape drives. A proxy database maps remote
users to local ones, as DECnet proxies did. SSH replaced them.

vaxpunk has none.
