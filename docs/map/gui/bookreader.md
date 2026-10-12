+++
title = "Bookreader"
summary = "Reads DEC's online books: shelves, hypertext links, figures, search"
aka = ["Bookreader", "DECW$BOOKREADER", "DECW$BOOK", "DECW$BOOKSHELF", "online documentation"]
kind = "layered"
intent = "aspiration"
research = "deferred"
status = "todo"

[[needs]]
on = "/gui/motif"
why = "Its windows are Motif"

[[needs]]
on = "/rms/core"
why = "Reads book and shelf files"

[[needs]]
on = "/layered/vax-document"
why = "VAX DOCUMENT wrote the books"
soft = true

[[needs]]
fuzzy = "The book and bookshelf file formats, and books to read"
why = "What it shows"
+++

Bookreader showed DEC's documentation on the screen: bookshelves of
books, each a `.DECW$BOOK` file with a table of contents, pages,
figures and hypertext links, searched and printed from the window. DEC
shipped the whole VMS documentation set this way on CD-ROM, written
with VAX DOCUMENT. Display PostScript figures went with Adobe's support
in 1998.

vaxpunk has none.
