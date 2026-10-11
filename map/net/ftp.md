+++
title = "FTP"
summary = "File transfer: the FTP client, the server, anonymous FTP and COPY/FTP"
aka = ["FTP", "TCPIP$FTP", "COPY/FTP", "TFTP", "anonymous FTP"]
kind = "core"
intent = "aspiration"
research = "outline"

[[feature]]
id = "client"
title = "The FTP client, and COPY/FTP"
status = "todo"

[[feature]]
id = "server"
title = "The FTP server, logins checked against the UAF"
status = "todo"
needs = ["//utils/loginout"]

[[feature]]
id = "anonymous"
title = "Anonymous FTP from a restricted directory"
status = "todo"

[[feature]]
id = "tftp"
title = "TFTP, for diskless boots"
status = "todo"

[[needs]]
on = "//net/tcpip-qio"
why = "A control connection and a data connection per transfer"

[[needs]]
on = "//rms/core"
why = "Text and binary transfers map onto record formats"

[[needs]]
on = "//net/aux-server"
why = "The server is started per connection"
soft = true
+++

TCP/IP Services' FTP client takes VMS or UNIX file names and maps text
and binary transfers onto RMS record formats; `COPY/FTP` does the same
from DCL. Its server logs users in against the UAF, can offer anonymous
FTP from a restricted directory, and speaks both VMS and UNIX directory
listings. TFTP and BOOTP serve diskless clients.

vaxpunk has none yet. FTP would be one way to the "VMS-to-VMS remote
file" goal of PRD-0002, and the one that also reaches every other
system.
