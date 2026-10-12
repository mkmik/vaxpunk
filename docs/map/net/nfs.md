+++
title = "NFS"
summary = "NFS server and client, the portmapper, and PC-NFS"
aka = ["NFS", "TCPIP$NFS", "portmapper", "PC-NFS", "ONC RPC", "MOUNT /NFS"]
kind = "core"
intent = "aspiration"
research = "deferred"

[[feature]]
id = "rpc"
title = "ONC RPC and the portmapper"
status = "todo"

[[feature]]
id = "server"
title = "NFS server: export VMS directories, map UNIX users to UICs"
status = "todo"

[[feature]]
id = "client"
title = "NFS client: TCPIP MOUNT of a UNIX file system as a VMS disk"
status = "todo"

[[feature]]
id = "pc-nfs"
title = "PC-NFS authentication"
status = "todo"

[[needs]]
on = "/net/tcpip-qio"
why = "UDP and TCP RPC"

[[needs]]
on = "/files/ods5"
why = "UNIX names, case and attributes need ODS-5"

[[needs]]
on = "/rms/core"
why = "Record formats seen as byte streams"

[[needs]]
fuzzy = "A proxy database of UNIX UIDs and GIDs to VMS users"
why = "Maps NFS's identities to UICs"
+++

TCP/IP Services exports VMS directories to UNIX systems with an NFS
server, mapping each UNIX user to a VMS one through its proxy database
and converting record files to byte streams; its NFS client mounts a
UNIX file system as a VMS disk device with `TCPIP MOUNT`. Both run on
ONC RPC and the portmapper, and PC-NFS clients authenticate through it
too. NFS was a common way for VMS and UNIX systems to share files.

vaxpunk has none.
