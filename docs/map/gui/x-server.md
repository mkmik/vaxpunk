+++
title = "Local X server"
summary = "The DECwindows X display server, drawing on the workstation's own screen"
aka = ["X server", "DECW$SERVER", "DECwindows X11 Display Server", "DECW$SERVER_0_ERROR.LOG", "DECW$PRIVATE_SERVER_SETUP", "Xvnc"]
kind = "layered"
intent = "aspiration"
research = "deferred"
status = "dropped"
refs = ["https://h41379.www4.hpe.com/doc/82final/6300/6300pro_002.html"]

[[needs]]
on = "/drivers/graphics"
why = "What a local server draws on and reads from"
+++

On a VMS workstation the X display server is a detached process,
started by `DECW$STARTUP`, that owns the graphics card, keyboard and
mouse and draws for every client, local or remote. It is a port of the
X11R6 server with extensions loaded on demand, its transports and
access lists set in `DECW$PRIVATE_SERVER_SETUP.COM`, and its log in
`DECW$SERVER_0_ERROR.LOG`. A VMS server with no graphics device runs
only clients.

Dropped for vaxpunk: Marko's plan has no display drivers. The X server
is remote, Xvnc on the Mac, and vaxpunk runs only clients.
