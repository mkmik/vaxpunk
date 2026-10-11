+++
title = "DCX$ data compression"
summary = "Compress and expand records, as the librarian's /DATA=REDUCE does"
aka = ["DCX$", "DCX$ANALYZE_INIT", "DCX$MAKE_MAP", "DCX$COMPRESS", "DCX$EXPAND"]
kind = "core"
intent = "aspiration"
research = "deferred"
status = "todo"

[[needs]]
on = "/exec/calling-standard"
why = "Called from any language by the standard"
+++

The Data Compression/Expansion routines analyze a set of records, build
a compression map from their statistics, and compress and expand
records with it. The librarian uses them for `LIBRARY /DATA=REDUCE`,
which shrinks text and help libraries; programs can use them for their
own data.

vaxpunk has none.
