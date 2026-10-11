+++
title = "TDMS"
summary = "Terminal Data Management System: VT forms run by requests, before DECforms"
aka = ["TDMS", "VAX TDMS", "Terminal Data Management System", "TSS$", "request library", ".RLB", "RDU", "FDU"]
kind = "layered"
intent = "aspiration"
research = "deferred"
status = "todo"
refs = ["https://h41379.www4.hpe.com/doc/721final/6609/6609pro_007.html", "https://www.kednos.com/pli_examples/00913B19-C7B51640-33014E.HTML"]

[[needs]]
on = "//drivers/terminal"
why = "Forms on VT terminals"

[[needs]]
on = "//layered/cdd"
why = "Forms, requests and records are defined in the dictionary"

[[needs]]
on = "//exec/calling-standard"
why = "TSS$ calls from any language"

[[needs]]
on = "//rms/core"
why = "Request library files"
+++

TDMS, the VAX Terminal Data Management System, is the forms product ACMS
used before DECforms. Forms and requests are defined in CDD; a request
says which form to show and how its fields map to a program record, and
requests are built into a request library, an `.RLB` file. A program
opens a channel and the library and calls `TSS$REQUEST` to run one.
ACMS still supports TDMS exchange steps for older applications, but DEC
recommended DECforms for new ones, and DECforms has a TDMS converter.

On vaxpunk it is an aspiration. ACMS can use it instead of DECforms.
