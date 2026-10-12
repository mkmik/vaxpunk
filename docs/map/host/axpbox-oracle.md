+++
title = "OpenVMS Alpha oracle (AXPbox)"
summary = "OpenVMS Alpha in the AXPbox emulator, scripted, to check vaxpunk against"
aka = ["AXPbox", "run-vms.py", "bliss-oracle.py", "tpu-oracle.py", "check.py", "OpenVMS Alpha 8.4-2L1", "ES40"]
kind = "tool"
intent = "goal"
research = "done"
refs = ["crosstools/ods/vms/run-vms.py", "crosstools/ods/vms/setup.sh", "crosstools/ods/vms/check.py", "crosstools/ods/vms/bliss-oracle.py", "crosstools/ods/vms/tpu-oracle.py", "crosstools/ods/docs/vms-check.md", "docs/prd/0004-bliss64-compiler.md", "docs/prd/0006-tpu-and-eve.md", "docs/prd/0008-rms-record-and-indexed-files.md"]

[[feature]]
id = "harness"
title = "run-vms.py: boot the install CD, type DCL commands, shut down cleanly"
status = "done"
note = "Retries the one run in six that hangs"

[[feature]]
id = "volumes"
title = "VMS checks ods's volumes and BACKUP copies them"
status = "done"
note = "vms/check.py; ods step 9"

[[feature]]
id = "bliss"
title = "BLISSA64's listings and program output as expected results"
status = "done"
note = "PRD-0004 step 3; bliss-oracle.py"

[[feature]]
id = "tpu"
title = "EDIT/TPU's output and screens for scripted sessions"
status = "done"
note = "PRD-0006; tpu-oracle.py"

[[feature]]
id = "rms"
title = "Relative and indexed files made and dumped by OpenVMS, as fixtures"
status = "done"
note = "crosstools/ods/fixtures/rms; PRD-0008"

[[feature]]
id = "macro32"
title = "MACRO/MIGRATION builds, for WASMRUN's sources on Alpha"
status = "todo"
note = "PRD-0007 M9"

+++

Not a VMS piece: how vaxpunk asks real VMS what it would do. `setup.sh`
builds the AXPbox Alpha emulator (GPL, cloned locally, never committed)
and fetches an AlphaServer ES40's
firmware, and `run-vms.py` boots OpenVMS Alpha V8.4-2L1 on its own copy
of the disk, types DCL commands, and shuts down. On top of it, scripts
compile BLISS with DEC's BLISSA64, run TPU sessions, make RMS files and
check `ods`'s volumes. Results are committed as expected outputs, so CI
never needs AXPbox. The oracle answers for behaviour and listings, never
for machine code.
