+++
title = "ANALYZE/RMS_FILE"
summary = "Checks an RMS file's structure, reports its statistics, writes its FDL"
aka = ["ANALYZE/RMS_FILE", "ANLRMS", "ANALYZRMS.EXE", "ANALYZE/RMS_FILE/CHECK", "ANALYZE/RMS_FILE/FDL", "ANALYZE/RMS_FILE/INTERACTIVE"]
kind = "core"
intent = "goal"
research = "done"
refs = ["vms/sysexe/analyzrms.mar", "vms/cld/analyze.cld", "docs/prd/0008-rms-record-and-indexed-files.md", "https://docs.vmssoftware.com/vsi-openvms-record-management-utilities-reference-manual/"]

[[feature]]
id = "check"
title = "/CHECK: walk each key's index and data buckets and report what is wrong"
status = "done"
note = "PRD-0008 step 7; the report matched the oracle's on every fixture"

[[feature]]
id = "statistics"
title = "/STATISTICS and /SUMMARY: each area's and key's figures"
status = "done"
note = "PRD-0008 step 7"

[[feature]]
id = "fdl"
title = "/FDL: an FDL file that makes the file again, with ANALYSIS_OF_KEY"
status = "done"
note = "PRD-0008 step 7"

[[feature]]
id = "output"
title = "/OUTPUT, and pages of 54 lines under a heading"
status = "done"

[[feature]]
id = "deeper-checks"
title = "Keys' order in buckets, a relative file's cells, a sequential file's records"
status = "todo"
note = "analyzrms.mar ponytail"

[[feature]]
id = "big-buckets"
title = "Buckets larger than MAXBKT blocks"
status = "todo"
needs = ["//rms/indexed:big-buckets"]
note = "analyzrms.mar ponytail"

[[feature]]
id = "interactive"
title = "/INTERACTIVE: walk the structure a bucket at a time"
status = "todo"
needs = ["//drivers/terminal"]
note = "PRD-0008 non-goal"

[[feature]]
id = "edit-fdl"
title = "EDIT/FDL, the FDL editor that tunes a design from /FDL's figures"
status = "todo"
needs = ["//rms/fdl:edit-fdl"]
note = "PRD-0008 non-goal"

[[needs]]
on = "//rms/indexed"
why = "Reads the prologue, areas, keys and buckets"

[[needs]]
on = "//rms/relative"
why = "A relative file's prologue"

[[needs]]
on = "//rms/sequential"
why = "Reads files by block and writes the report"

[[needs]]
on = "//rms/fdl"
why = "/FDL writes the FDL language"

[[needs]]
on = "//rtl/cli"
why = "Its parameters and qualifiers"
+++

`ANALYZE/RMS_FILE` reads an RMS file's structure directly and reports
on it. `/CHECK` walks every index and data bucket of an indexed file and
says what is broken; `/STATISTICS` gives fill factors, index depth and
compression for each key; `/FDL` writes an FDL file with those figures,
which `EDIT/FDL` uses to tune bucket sizes and `CONVERT/FDL` to rebuild
the file. `/INTERACTIVE` lets a user walk the structure bucket by
bucket. The same utility is the usual judge of whether a file is
corrupt.

vaxpunk's is `vms/sysexe/analyzrms.mar` (PRD-0008 step 7): headers, RMS
attributes, prologues, areas and keys; `/CHECK`, `/STATISTICS`,
`/SUMMARY`, `/FDL` and `/OUTPUT`. Its reports matched OpenVMS Alpha's
on PRD-0008's fixtures. Of an indexed file it checks what a reader needs
to find its way, not the order of keys; `/INTERACTIVE` and `EDIT/FDL`
are PRD-0008 non-goals.
