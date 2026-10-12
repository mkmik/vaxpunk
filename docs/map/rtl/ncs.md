+++
title = "NCS$ national character sets"
summary = "Collating sequences and conversion tables for languages other than English"
aka = ["NCS$", "NCS", "NCS$COMPARE", "NCS$CONVERT", "collating sequence"]
kind = "core"
intent = "aspiration"
research = "deferred"
status = "todo"

[[needs]]
on = "/rtl/sor"
why = "SORT orders records by a national collating sequence"
soft = true

[[needs]]
fuzzy = "The NCS utility and its library of collating sequences and conversion functions"
why = "Where the tables are defined and kept"
+++

The National Character Set utility (`NCS`) defines collating sequences
and conversion functions for languages and character sets other than
plain ASCII, kept in a library, and the `NCS$` routines compare and
convert strings with them. SORT and RMS indexed keys can collate by
them, so that, say, Spanish names sort as Spanish readers expect.

vaxpunk has none; everything collates as ASCII.
