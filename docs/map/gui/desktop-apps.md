+++
title = "DECwindows applications"
summary = "Calculator, Calendar, Clock, Mail, Notepad, Paint and the other desk tools"
aka = ["DECW$CALC", "DECW$CALENDAR", "DECW$CLOCK", "DECW$MAIL", "DECW$NOTEPAD", "DECW$PAINT", "DECW$PUZZLE", "Print Screen", "Message Window"]
kind = "layered"
intent = "aspiration"
research = "deferred"
status = "todo"
refs = ["https://h50146.www5.hpe.com/products/software/oe/openvms/manual/v73-1/pdfs/jmotif_v13_spd.pdf"]

[[needs]]
on = "/gui/motif"
why = "They are Motif programs"

[[needs]]
on = "/gui/dxm"
why = "Help and colour widgets"
soft = true

[[needs]]
on = "/utils/mail"
why = "DECwindows Mail is a window on VMS MAIL's files"
soft = true
+++

DECwindows Motif came with a set of small applications: Calculator,
Calendar, Clock, Mail (a window on VMS MAIL's folders), Notepad, Paint,
Print Screen, Puzzle, a message window, and a tutorial. They show what a
VMS workstation's desk looked like, and make good first programs for a
new toolkit.

vaxpunk has none.
