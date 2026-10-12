+++
title = "DECwindows"
band = "apps"
summary = "The windowing system: X11 clients, Motif and the DECwindows applications"
+++

DECwindows is VMS's graphical user interface, the X Window System as
DEC packaged it. It replaced VWS, the VAXstation's own windowing system,
in 1989; DEC's first toolkit, XUI, gave way to Motif in 1991, making
DECwindows Motif, a layered product. Its layers, bottom up: a transport
(`SET DISPLAY`, over TCP/IP, DECnet, LAT or shared memory), Xlib, the X
Toolkit Intrinsics, the Motif toolkit and DEC's extensions to it (DXm),
UIL and MRM for interfaces described apart from code; then the Session
Manager, the Motif Window Manager, DECterm, FileView, Bookreader and
smaller applications, and since V7.1 the CDE-based New Desktop. On a
workstation the X display server runs on VMS too; any other X server,
an X terminal or a PC, can show the same applications.

vaxpunk has none of it yet. Marko's plan has no display drivers: VMS X11
clients talk to a remote X server, such as Xvnc on the Mac, so the work
is the client side, Xlib, Xt, Motif, UIL and MRM, and maybe a VUIT-like
builder. Everything here is an aspiration.
