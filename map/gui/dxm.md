+++
title = "DXm"
summary = "DECwindows extensions to Motif: Help, Color Mix, CSText, Print, SVN widgets"
aka = ["DXm", "DECwindows Extensions to Motif", "DECW$DXMLIBSHR12", "SVN", "Structured Visual Navigation", "Color Mix", "CSText", "Help widget"]
kind = "layered"
intent = "aspiration"
research = "deferred"
status = "todo"
refs = ["https://h50146.www5.hpe.com/products/software/oe/openvms/manual/v73-1/pdfs/jmotif_v13_spd.pdf"]

[[needs]]
on = "//gui/motif"
why = "Its widgets are Motif widgets"

[[needs]]
on = "//rtl/lbr"
why = "The Help widget reads help libraries"
soft = true
+++

DXm is what DEC added to Motif for its own applications: a Help widget
that shows hypertext help from VMS help libraries, Color Mix for picking
colours, Compound String Text (CSText) for text in mixed character sets
and directions, a Print widget, and SVN, Structured Visual Navigation,
for outlines and trees. The DECwindows applications use them, and so
could programs. VSI calls them VSI Extensions to Motif.

vaxpunk has none.
