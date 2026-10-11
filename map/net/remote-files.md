+++
title = "VMS-to-VMS file access"
summary = "Opening files on another VMS by node::file, as DECnet's DAP and FAL did"
aka = ["DAP", "FAL", "node::", "remote RMS"]
kind = "core"
intent = "goal"
research = "deferred"
status = "todo"
refs = ["docs/prd/0002-networking.md"]

[[needs]]
on = "//net/tcpip-qio"
why = "Over TCP, since there is no DECnet"

[[needs]]
on = "//rms/remote"
why = "RMS sends node:: file operations to the remote side"

[[needs]]
on = "//utils/loginout"
why = "The server side checks the user against the UAF"

[[needs]]
fuzzy = "A protocol: DAP over TCP, FTP, NFS or something VMS-flavoured"
why = "PRD-0002 leaves the choice open"
+++

With DECnet, any VMS program could open `NODE"user password"::DKA0:[DIR]FILE.TXT`:
RMS sent the operation in DAP, the Data Access Protocol, to FAL, the
File Access Listener, on the other node, which did it there. `COPY`,
`TYPE`, `DIRECTORY` and every program that used RMS worked across the
network unchanged. TCP/IP has nothing that does all of that; FTP, NFS
and rcp each cover part.

PRD-0002 names VMS-to-VMS remote file access as one of the
applications TCP/IP should unblock, and leaves the protocol open.
DECnet itself is dropped. `COPY/HTTP` already borrows the `node::"path"`
syntax for a web server.
