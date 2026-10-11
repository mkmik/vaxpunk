+++
title = "Window manager"
summary = "The Motif Window Manager, MWM, and the New Desktop's DTWM"
aka = ["MWM", "Motif Window Manager", "DECW$MWM", "DTWM", "window manager"]
kind = "layered"
intent = "aspiration"
research = "deferred"
status = "todo"

[[needs]]
on = "//gui/motif"
why = "MWM is a Motif program"

[[needs]]
fuzzy = "ICCCM's window manager conventions, and MWM's own hints"
why = "How clients and the window manager agree"
+++

A window manager draws the frames around other programs' windows and
lets the user move, resize, iconify and stack them. DECwindows had its
own until the Motif Window Manager, MWM, became the default in VMS V6.0;
the New Desktop uses DTWM, CDE's, and the Session Manager's Customize
menu can name another vendor's.

vaxpunk has none. With the X server on the Mac, a window manager could
run on either side.
