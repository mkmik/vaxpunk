+++
title = "CDA"
summary = "Compound Document Architecture: DDIF and DTIF documents, and the CDA Viewer"
aka = ["CDA", "Compound Document Architecture", "DDIF", "DTIF", "CDA Viewer", "DDIF$VIEWSHR", "CDA$ACCESS", "DEClinks"]
kind = "layered"
intent = "aspiration"
research = "deferred"
status = "todo"

[[needs]]
on = "//gui/motif"
why = "The viewer's window is Motif"

[[needs]]
on = "//rms/core"
why = "Documents are files"

[[needs]]
fuzzy = "DDIF and DTIF formats, the CDA toolkit, and converters to and from other formats"
why = "What it reads and writes"
+++

CDA, the Compound Document Architecture, was DEC's way for programs to
share documents with text, graphics and images: DDIF for documents and
DTIF for tables, a toolkit that reads and writes them, converters to
and from other formats, and the CDA Viewer, which shows them in a
DECwindows window or as text on a terminal. DECwrite and DECdecision
were built on it; DEClinks linked pieces of documents together.

vaxpunk has none.
