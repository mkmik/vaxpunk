+++
title = "CONV$ callable convert"
summary = "The routines behind CONVERT: copy records between file organizations"
aka = ["CONV$", "CONVSHR", "CONV$PASS_FILES", "CONV$CONVERT", "CONV$RECLAIM"]
kind = "core"
intent = "aspiration"
research = "outline"
status = "todo"

[[needs]]
on = "/rms/indexed"
why = "Builds and reclaims indexed files"

[[needs]]
on = "/rms/fdl"
why = "Takes the output's attributes from an FDL file"

[[needs]]
on = "/rtl/sor"
why = "Sorts records by the primary key before loading an indexed file"
+++

`CONV$` is the CONVERT utility as a library: `CONV$PASS_FILES`,
`CONV$PASS_OPTIONS` and `CONV$CONVERT` copy records from one or more
input files into an output of any organization, as `CONVERT` does, and
`CONV$RECLAIM` reclaims empty buckets of an indexed file in place, as
`CONVERT/RECLAIM`.

vaxpunk's `CONVERT` is a program, `vms/sysexe/convert.mar`, with no
callable interface.
