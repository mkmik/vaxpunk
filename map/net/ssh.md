+++
title = "SSH"
summary = "Secure remote login, scp and sftp: client and server"
aka = ["SSH", "TCPIP$SSH", "OpenSSH", "scp", "sftp", "SSH2"]
kind = "core"
intent = "aspiration"
research = "deferred"

[[feature]]
id = "client"
title = "ssh client: a remote login from a VMS terminal"
status = "todo"

[[feature]]
id = "server"
title = "sshd: logins on a pseudo-terminal through LOGINOUT"
status = "todo"
needs = ["//utils/loginout", "//drivers/pseudo-terminal"]

[[feature]]
id = "sftp"
title = "scp and sftp, with VMS file names and record formats"
status = "todo"

[[needs]]
on = "//net/tcpip-qio"
why = "Runs over a TCP connection"

[[needs]]
on = "//exec/entropy"
why = "Keys and nonces"

[[needs]]
fuzzy = "SSH's ciphers, key exchange and host keys: an OpenSSH port on the C library, or a lighter implementation"
why = "Marko is weighing an OpenSSH port against a lighter one"

[[needs]]
on = "//rtl/crtl"
why = "An OpenSSH port would be C"
soft = true
+++

TCP/IP Services added SSH in its V5 releases: `ssh`, `scp` and `sftp`
clients and a server, whose logins get a terminal and go through
LOGINOUT. VSI later ships an OpenSSH port. Process Software sold its own
for MultiNet and TCPware. SSH replaced
TELNET, rlogin and FTP on any system reachable from outside.

Marko is looking at SSH for vaxpunk now: an OpenSSH port on the C
run-time library, or a lighter implementation. Nothing is built; the
choice decides most of what it needs.
