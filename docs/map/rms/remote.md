+++
title = "Remote file access"
summary = "RMS sends node:: operations to a server on the other node, as DAP and FAL did"
aka = ["node::", "DAP", "FAL", "Data Access Protocol", "File Access Listener", "access control string", "NETPROXY", "SET RMS_DEFAULT/NETWORK_BLOCK_COUNT"]
kind = "core"
intent = "goal"
research = "deferred"
refs = ["docs/prd/0002-networking.md", "docs/prd/0008-rms-record-and-indexed-files.md", "vms/sysexe/copy.mar", "https://wasd.vsm.com.au/htbin/helpgate/HELP/SET/RMS_DEFAULT/QUALIFIERS"]

[[feature]]
id = "parse"
title = "$PARSE takes node:: and node\"user password\":: in a specification"
status = "todo"
note = "PRD-0002 goal: VMS-to-VMS remote file access; COPY/HTTP parses node:: itself"

[[feature]]
id = "operations"
title = "$OPEN, $CREATE, $GET, $PUT, $SEARCH, $ERASE on a remote file, sent over the network"
status = "todo"
needs = ["#parse", "/net/tcpip-qio"]

[[feature]]
id = "proxies"
title = "Proxy logins: a remote user mapped to a local one without a password"
status = "todo"
needs = ["#operations"]

[[feature]]
id = "dap-decnet"
title = "DAP over DECnet to FAL, as VMS does it"
status = "dropped"
note = "PRD-0002: DECnet dropped"

[[needs]]
on = "/rms/core"
why = "The FAB and RAB services, sent to another node instead of the XQP"

[[needs]]
on = "/net/tcpip-qio"
why = "Over TCP, since there is no DECnet"

[[needs]]
fuzzy = "A file access protocol and its server: DAP over TCP, or something of vaxpunk's own"
why = "PRD-0002 leaves the protocol open"
+++

On a DECnet node any RMS specification could start with a node name,
`NODE"user password"::DKA0:[DIR]FILE.TXT`. RMS then sent the operation in
the Data Access Protocol (DAP) to the File Access Listener (FAL), a
process that the network started on the other node under that user, or
under a proxy account from `NETPROXY.DAT`, which did it there and sent
back records and statuses. `COPY`, `TYPE`, `DIRECTORY`, `DELETE` and
every program that used RMS worked across the network unchanged.
`SET RMS_DEFAULT/NETWORK_BLOCK_COUNT` sized the buffers.

vaxpunk has no remote RMS. DECnet is dropped, and
[PRD-0002](../../prd/0002-networking.md) names VMS-to-VMS remote
file access over TCP/IP as an application to build, leaving the protocol
open (`t8y:/net/remote-files`). `COPY/HTTP` borrows the `node::"path"`
syntax for web servers, parsing it in `COPY` itself. PRD-0008 leaves
`NODE::` to PRD-0002.
