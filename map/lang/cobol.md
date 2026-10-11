+++
title = "COBOL"
summary = "VAX COBOL, DEC COBOL and VSI COBOL: business programs on RMS files"
aka = ["VAX COBOL", "DEC COBOL", "Compaq COBOL", "HP COBOL", "VSI COBOL", "COBOL-85", ".COB"]
kind = "layered"
intent = "aspiration"
research = "deferred"
status = "todo"
refs = ["https://llvm.org/devmtg/2017-10/slides/Reagan-Porting%20OpenVMS%20Using%20LLVM.pdf", "https://docs.vmssoftware.com/"]

[[needs]]
on = "//rtl/cobol-rtl"
why = "ACCEPT, DISPLAY, file I/O and its status codes"

[[needs]]
on = "//rms/indexed"
why = "Indexed, relative and sequential files are the language's data"

[[needs]]
on = "//exec/calling-standard"
why = "Calls and is called by the standard"

[[needs]]
on = "//layered/cdd"
why = "COPY FROM DICTIONARY takes record definitions from CDD"
soft = true

[[needs]]
on = "//layered/dbms"
why = "COBOL's data manipulation language for DBMS"
soft = true

[[needs]]
fuzzy = "A compiler that runs on vaxpunk, with packed decimal arithmetic"
why = "Decimal is COBOL's arithmetic; VSI's x86-64 port had packed decimal left to map onto LLVM"
+++

VAX COBOL, then DEC COBOL on Alpha, renamed Compaq, HP and VSI COBOL,
is ANSI COBOL-85 with VMS extensions: RMS indexed and relative files
with record locking, `COPY FROM DICTIONARY` for CDD records, a data
manipulation language for DBMS, and calls by the standard. Banks,
insurers and manufacturers ran their transaction systems on it, often
under ACMS with forms in TDMS or DECforms. VSI ships it for x86-64. It
is a layered product.

vaxpunk has none. Its decimal arithmetic would need what MACRO-32 lacks
too: packed decimal.
