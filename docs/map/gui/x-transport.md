+++
title = "SET DISPLAY and transports"
summary = "How a DECwindows client reaches its X server: WSA devices and transports"
aka = ["SET DISPLAY", "SHOW DISPLAY", "DECW$DISPLAY", "WSA", "WSAn:", "DECW$TRANSPORT", "/TRANSPORT=TCPIP", "X authority", "DISPLAY"]
kind = "layered"
intent = "aspiration"
research = "outline"
refs = ["https://h41379.www4.hpe.com/doc/84final/9996/9996pro_186.html", "https://www.raymii.org/s/blog/OpenVMS_CDE_Desktop_remote_x_axpbox.html"]

[[feature]]
id = "set-display"
title = "SET DISPLAY /CREATE makes a WSAn: device and DECW$DISPLAY names it"
status = "todo"
needs = ["/exec/logical-names", "/dcl/cld"]

[[feature]]
id = "tcpip"
title = "The TCP/IP transport: X11 over TCP to the server's port"
status = "todo"
needs = ["/net/tcpip-qio"]

[[feature]]
id = "xauth"
title = "Connects with the X server's authorization cookie"
status = "todo"

[[feature]]
id = "local"
title = "The LOCAL transport: shared memory to a server on the same node"
status = "dropped"
needs = ["/exec/global-sections", "/gui/x-server"]
note = "Only for a local X server, which is dropped: no display drivers"

[[feature]]
id = "lat"
title = "The LAT transport, for X terminals"
status = "todo"
note = "Needs LAT, t8y:/net/lat"

[[feature]]
id = "decnet"
title = "The DECnet transport, DECwindows's default"
status = "dropped"
needs = ["/net/decnet"]
note = "DECnet is dropped (PRD-0002)"

[[needs]]
on = "/net/lat"
why = "The LAT transport"
soft = true

[[needs]]
on = "/net/dns-resolver"
why = "/NODE takes a host name"
soft = true

[[needs]]
on = "/net/ssh"
why = "X11 forwarded through an SSH connection"
soft = true

[[needs]]
fuzzy = "The WSA pseudo-device driver, and how Xlib finds its transport from it"
why = "What SET DISPLAY creates"

[[needs]]
fuzzy = "A remote X server, Xvnc on the Mac"
why = "Where the windows are drawn"
+++

A DECwindows client finds its display through a device, not an
environment variable: `SET DISPLAY /CREATE /NODE=host /TRANSPORT=TCPIP
/SERVER=n` creates a workstation pseudo-device, `WSAn:`, holding the node,
server, screen and transport, and points the logical name
`DECW$DISPLAY` at it; `SHOW DISPLAY` shows it and `SET DISPLAY /DELETE`
removes it. Xlib opens the display that names and loads the transport:
DECnet (the default), TCP/IP, LAT for X terminals, or LOCAL, shared
memory to the X server on the same workstation. TCP/IP Services and
MultiNet supply the TCP/IP one. Who may connect is the server's
business: its access list, which the Session Manager edits, and an X
authority file.

vaxpunk has none of it. In Marko's plan TCP/IP is the transport that
matters, since the X server is remote: Xvnc on the Mac. The sockets
`$QIO` interface is there for it (`t8y:/net/tcpip-qio`).
