+++
title = "STR$ routines"
summary = "String manipulation on descriptors: copy, concatenate, compare, search"
aka = ["STR$", "STR$COPY_DX", "STR$CONCAT", "STR$TRIM", "STR$UPCASE", "dynamic strings"]
kind = "core"
intent = "aspiration"
research = "outline"
status = "todo"
refs = ["https://h41379.www4.hpe.com/doc/84final/4493/4493pro_003.html"]

[[needs]]
on = "/rtl/librtl"
why = "Shares LIBRTL.EXE and its virtual memory routines"

[[needs]]
on = "/exec/calling-standard"
why = "Strings by descriptor: fixed, varying and dynamic"
+++

`STR$` is the string part of the run-time library: routines that copy,
concatenate, compare, trim, upcase, search and translate strings given
by descriptor, including dynamic strings whose storage the library
allocates and frees, and decimal string arithmetic (`STR$ADD`,
`STR$MUL`). BASIC, COBOL and DCL-like programs lean on it; any language
can call it.

vaxpunk has none. Its programs do strings in MACRO-32 and BLISS-64
themselves.
