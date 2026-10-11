+++
title = "DTM"
summary = "DECset's Test Manager: regression tests run and compared to benchmarks"
aka = ["DTM", "DEC Test Manager", "Digital Test Manager", "DECset", "test collection", ".SESSION", ".BMK"]
kind = "layered"
intent = "aspiration"
research = "deferred"
status = "todo"
refs = ["https://docs.vmssoftware.com/vsi-decset-for-openvms-guide-to-vsi-digital-test-manager/", "https://docs.vmssoftware.com/vsi-dec-test-manager-v457-release-notes/"]

[[needs]]
on = "/dcl/procedures"
why = "Tests, prologues and epilogues are command procedures"

[[needs]]
on = "/dcl/spawn"
why = "Records and plays tests in a subprocess"

[[needs]]
on = "/utils/job-controller"
why = "Collections can run in batch"
soft = true

[[needs]]
on = "/utils/differences"
why = "Results are compared with the benchmark"
soft = true

[[needs]]
fuzzy = "Recording a terminal session and playing it back to the program as typed input"
why = "Interactive terminal tests"

[[needs]]
on = "/gui/motif"
why = "DECwindows tests and its own interface"
soft = true

[[needs]]
on = "/drivers/pseudo-terminal"
why = "Runs an interactive test on a pseudo terminal, typing the recorded input"
+++

DTM, the DEC Test Manager, is DECset's regression test tool. A test
library holds test descriptions: a template (a command procedure, or a
recorded terminal or DECwindows session), a benchmark of the expected
output, prologue and epilogue, variables and filters. Tests run in a
collection, interactively or in batch; each result is filtered and
compared with its benchmark, and a difference file is kept where they
disagree. Terminal sessions are recorded once and played back as the
program reads input.

On vaxpunk it is an aspiration. vaxpunk's own tests run from the host
(`boot-runner`, `vrun`) and from STARTUP.
