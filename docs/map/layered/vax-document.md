+++
title = "VAX DOCUMENT"
summary = "DEC's text formatter: SDML tags through a modified TeX to print and online"
aka = ["VAX DOCUMENT", "DECdocument", "SDML", "DOCUMENT", "doctype", "DECTEX"]
kind = "layered"
intent = "aspiration"
research = "deferred"
status = "todo"
refs = ["https://ntg.nl/maps/02/05.pdf", "https://bitsavers.org/pdf/dec/vax/vms/vax_document/"]

[[needs]]
on = "/rms/core"
why = "Reads SDML files and writes output and book files"

[[needs]]
on = "/dcl/cld"
why = "The DOCUMENT command and its doctype and destination arguments"

[[needs]]
on = "/utils/print-symbiont"
why = "Output goes to PostScript and LN03 printers"
soft = true

[[needs]]
on = "/gui/bookreader"
why = "Online books in Bookreader format"
soft = true

[[needs]]
fuzzy = "A TeX engine with DEC's changes, and fonts for PostScript and LN03"
why = "The text processor under the tags"
+++

VAX DOCUMENT, later DECdocument, is the formatter DEC wrote its own
manuals with. Authors mark up text in SDML, Standard Digital Markup
Language, tags such as `<chapter>` and `<p>`; a doctype picks the
layout (manual, report, overhead, ...), and a destination picks the
output: PostScript, LN03, a line printer or, later, Bookreader online
books. Underneath, its text processor is TeX, modified by DEC. It builds
books from many files, with tables of contents, indexes and
cross-references across them. The VMS documentation set itself was
written in it.

On vaxpunk it is an aspiration. Nothing needs it.
