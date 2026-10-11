+++
title = "XDM"
summary = "XDMCP: X displays and X terminals get a DECwindows login from VMS"
aka = ["XDM", "XDMCP", "TCPIP$XDM", "X terminal"]
kind = "core"
intent = "aspiration"
research = "deferred"
status = "todo"

[[needs]]
on = "//net/tcpip-qio"
why = "XDMCP over UDP port 177, X over TCP"

[[needs]]
on = "//gui/session-manager"
why = "The login box and the session it starts"

[[needs]]
on = "//gui/xlib"
why = "Talks to the remote X server"
+++

TCP/IP Services' XDM server answers XDMCP queries from X terminals and
X servers on other machines, and shows them a DECwindows login box; the
user's session then runs on VMS, displayed there. It is how an office of
X terminals used a VMS system.

Marko's GUI plan is VMS X11 clients talking to a remote X server, such
as Xvnc on the Mac; with XDM, that server could ask vaxpunk for a login
(`Xvnc -query`) instead of a user starting clients by hand. Nothing is
built.
