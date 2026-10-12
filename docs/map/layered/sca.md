+++
title = "SCA"
summary = "DECset's Source Code Analyzer: cross-references and call graphs from compilers"
aka = ["SCA", "Source Code Analyzer", "DECset", "/ANALYSIS_DATA", ".ANA", "SCA library", "INSPECT"]
kind = "layered"
intent = "aspiration"
research = "outline"
status = "todo"
refs = ["https://docs.vmssoftware.com/vsi-decset-for-openvms-guide-to-vsi-source-code-analyzer/", "https://docs.vmssoftware.com/vsi-source-code-analyzer-v523-release-notes/"]

[[needs]]
fuzzy = "Compilers that write analysis data files (/ANALYSIS_DATA, .ANA)"
why = "SCA parses nothing itself; it loads what compilers write"

[[needs]]
on = "/rms/core"
why = "SCA libraries are files of loaded analysis data"

[[needs]]
on = "/gui/motif"
why = "Its DECwindows interface and graphical call graphs"
soft = true
+++

SCA, the Source Code Analyzer, is DECset's code browser. It doesn't parse
source itself: the DEC compilers write an analysis data file (`.ANA`)
when given `/ANALYSIS_DATA`, and `LOAD` merges those files into an SCA
library. Queries then find where a symbol is declared and used, show the
call graph and the structure of data types, and `INSPECT` checks that
declarations and calls agree across modules. LSE asks it for
`GOTO DECLARATION` and `FIND OCCURRENCES`.

On vaxpunk it is an aspiration. vaxpunk's compilers are host tools today
(vmacro, vbliss) and none writes analysis data.
