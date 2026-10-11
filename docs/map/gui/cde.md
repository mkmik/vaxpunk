+++
title = "CDE"
summary = "The New Desktop: DECwindows's CDE, with front panel, dtfile and dtterm"
aka = ["CDE", "New Desktop", "Common Desktop Environment", "dtsession", "dtwm", "dtfile", "dtlogin", "Front Panel", "ToolTalk", "CDE$SYSTEM_DEFAULTS"]
kind = "layered"
intent = "aspiration"
research = "deferred"
status = "todo"
refs = ["https://www.raymii.org/s/blog/OpenVMS_CDE_Desktop_remote_x_axpbox.html", "https://h41379.www4.hpe.com/doc/82final/6470/6470pro_001.html"]

[[needs]]
on = "/gui/motif"
why = "CDE's applications are Motif"

[[needs]]
on = "/gui/window-manager"
why = "DTWM"

[[needs]]
on = "/rtl/crtl"
why = "CDE came from UNIX and runs on the C library's UNIX side"

[[needs]]
fuzzy = "ToolTalk, actions and data types, the help system, the style manager"
why = "The parts of CDE beyond its applications"
+++

The New Desktop came with OpenVMS V7.1 in 1996, based on CDE, the Common
Desktop Environment that HP, IBM, Sun and Novell agreed on; DEC's
product description says it is not a full implementation. It has
`dtlogin` and `dtsession`, the DTWM window manager with its front panel,
the File Manager, Style Manager, Application Manager, Text Editor,
Calculator and Help Viewer, with DECterm alongside. It runs on Alpha and
Itanium; it was never on the VAX. Users choose it or the Traditional
Desktop at login.

vaxpunk has none.
