+++
title = "Motif"
summary = "The OSF/Motif toolkit (Xm): DECwindows's widget set"
aka = ["Motif", "OSF/Motif", "Xm", "DECW$XMLIBSHR12", "Motif 1.2", "Open Motif", "compound strings", "XmString"]
kind = "layered"
intent = "aspiration"
research = "outline"
refs = ["https://h50146.www5.hpe.com/products/software/oe/openvms/manual/v73-1/pdfs/jmotif_v13_spd.pdf"]

[[feature]]
id = "widgets"
title = "The widget set: shells, managers, buttons, lists, text, scroll bars, menus, dialogs"
status = "todo"
needs = ["//gui/xt:widgets"]

[[feature]]
id = "strings"
title = "Compound strings and font lists"
status = "todo"

[[feature]]
id = "clipboard"
title = "Clipboard, selections and drag and drop"
status = "todo"

[[needs]]
on = "//gui/xt"
why = "Motif's widgets are Xt widgets"

[[needs]]
fuzzy = "X fonts on the server, or a font server, with the families DECwindows expects"
why = "Every label and text widget needs fonts"

[[needs]]
fuzzy = "Which Motif: the Open Group's sources (LGPL since 2012), ported, or a rewrite"
why = "Not chosen"
+++

Motif is the widget toolkit the Open Software Foundation made in 1989
from DEC's XUI and HP's toolkit: windows with a common look, menus,
dialog boxes, lists, text fields and scroll bars, with compound strings
for text in any character set, and the clipboard. Its library is `Xm`.
DECwindows Motif ships Motif 1.2 (`DECW$XMLIBSHR12.EXE`), keeps running
programs linked against 1.1, and adds DEC's own widgets in DXm. Its
window manager is MWM. The Open Group released Motif under the LGPL in
2012.

vaxpunk has none. It is one of the layers Marko names as the work.
