+++
title = "DECwindows Motif"
summary = "VMS's X11 GUI product: libraries, desktops and applications"
aka = ["DECwindows", "DECwindows Motif", "DW-MOTIF", "DECW$STARTUP", "DECW$PRIVATE_APPS_SETUP", "Traditional Desktop", "New Desktop", "XUI"]
kind = "layered"
intent = "aspiration"
research = "outline"
refs = ["https://h50146.www5.hpe.com/products/software/oe/openvms/manual/v73-1/pdfs/jmotif_v13_spd.pdf", "https://h41379.www4.hpe.com/doc/82final/6470/6470pro_001.html", "https://docs.vmssoftware.com/"]

[[feature]]
id = "clients"
title = "Motif applications on vaxpunk, shown on a remote X server"
status = "todo"
needs = ["//gui/x-transport", "//gui/xlib", "//gui/xt", "//gui/motif"]
note = "Marko's plan: Xvnc on the Mac; no display drivers"

[[feature]]
id = "programming"
title = "The programming kit: UIL and MRM, DXm, headers, and bindings for other languages"
status = "todo"
needs = ["//gui/uil-mrm", "//gui/dxm"]

[[feature]]
id = "traditional-desktop"
title = "The Traditional Desktop: Session Manager, MWM, DECterm, FileView"
status = "todo"
needs = ["//gui/session-manager", "//gui/window-manager", "//gui/decterm", "//gui/fileview"]

[[feature]]
id = "applications"
title = "Bookreader, the desk applications and the CDA Viewer"
status = "todo"
needs = ["//gui/bookreader", "//gui/desktop-apps", "//gui/cda"]

[[feature]]
id = "new-desktop"
title = "The New Desktop, based on CDE"
status = "todo"
needs = ["//gui/cde"]

[[feature]]
id = "workstation"
title = "A workstation: the X display server on vaxpunk's own screen"
status = "dropped"
needs = ["//gui/x-server"]
note = "Marko: no display drivers"

[[needs]]
on = "//exec/shareable-images"
why = "Its libraries are shareable images every client links"

[[needs]]
on = "//utils/license-manager"
why = "DW-MOTIF is a licensed product"
soft = true

[[needs]]
fuzzy = "Language bindings: headers for C, Fortran, Pascal and Ada"
why = "Programs in each language call the libraries"
+++

DECwindows Motif is the layered product that gives VMS its GUI: Xlib,
Xt, Motif and DXm, UIL and MRM, the X display server for workstations,
the Session Manager and MWM, DECterm, FileView, Bookreader, Mail,
Calendar, Notepad, Paint and other applications, and since V7.1 the New
Desktop. `DECW$STARTUP` starts it, `DECW$PRIVATE_APPS_SETUP.COM` picks
the desktop, and `SET DISPLAY` points clients at any X server. Other
products have DECwindows faces: EVE under TPU, the debugger,
DECset's tools. VSI's current versions are 1.7 on Alpha and Itanium and
1.8 on x86-64.

vaxpunk has none. Marko's plan is clients only: no display drivers,
VMS X11 applications on vaxpunk drawing on a remote X server such as
Xvnc on the Mac, and the work in Xlib, Xt, Motif, UIL and MRM, maybe a
VUIT-like builder. The `clients` feature is that plan.
