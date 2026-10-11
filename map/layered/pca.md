+++
title = "PCA"
summary = "DECset's Performance and Coverage Analyzer: profiles and test coverage"
aka = ["PCA", "Performance and Coverage Analyzer", "DECset", "PCA$COLLECTOR", ".PCA", "profiler"]
kind = "layered"
intent = "aspiration"
research = "deferred"
status = "todo"
refs = ["https://docs.vmssoftware.com/vsi-decset-for-openvms-guide-to-performance-and-coverage-analyzer/", "https://docs.vmssoftware.com/vsi-decset-for-openvms-performance-and-coverage-analyzer-reference-manual/"]

[[needs]]
on = "//exec/image-activator"
why = "The Collector starts as the image's debugger, through LIB$DEBUG"

[[needs]]
on = "//utils/debugger:dst"
why = "Programs are compiled and linked for the debugger; it uses their symbol tables"

[[needs]]
on = "//exec/timers"
why = "Samples the program counter at a fixed interval, 10 ms by default"

[[needs]]
on = "//exec/exit-handlers"
why = "The Collector closes its data file at image exit"

[[needs]]
fuzzy = "Counting and coverage by breakpoints, and hooks for page faults, I/O and system services"
why = "The data besides time samples"

[[needs]]
on = "//gui/motif"
why = "The Analyzer's DECwindows histograms"
soft = true
+++

PCA, the Performance and Coverage Analyzer, is DECset's profiler. The
Collector runs inside the program, started by the image activator in
place of the debugger. It samples the program counter every 10 ms of
elapsed or CPU time, counts calls and lines, records which code ran, and
can log page faults, I/O and system service calls, all into a `.PCA`
file. The Analyzer then shows histograms, tables and annotated source
listings, on a terminal or in DECwindows.

On vaxpunk it is an aspiration. It needs the debugger's symbol tables,
and vaxpunk has no debugger yet.
