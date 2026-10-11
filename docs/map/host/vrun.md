+++
title = "vrun (image runner)"
summary = "Runs an image at EL0 in bare-metal QEMU, with console, files, fault reports"
aka = ["vrun", "runner-abi.md", "vrun.mlb", "$FOPEN", "$FREAD", "$FWRITE", "$FCLOSE"]
kind = "tool"
intent = "goal"
research = "done"
milestone = "m0"
refs = ["docs/prd/0001-vtools.md", "crosstools/vtools/crates/vrun", "crosstools/vtools/docs/runner-abi.md", "docs/prd/0004-bliss64-compiler.md", "docs/prd/0005-macro32-on-the-calling-standard.md"]

[[feature]]
id = "basic"
title = "An image at its link addresses, at EL0 behind a boot stub, exit status out"
status = "done"
milestone = "m0"
note = "PRD-0001 steps 2 and 3"

[[feature]]
id = "faults"
title = "Faults as %VRUN-F-ACCVIO with symbols from the map, stack overflow, --gdb"
status = "done"
milestone = "m0"
note = "PRD-0001 step 10"

[[feature]]
id = "traceback"
title = "A traceback through frames of the calling standard"
status = "done"
note = "PRD-0005 step 4"

[[feature]]
id = "files"
title = "File monitor calls, open, read, write, close, under TCG and HVF"
status = "done"
note = "PRD-0004 step 12, PR #95; a QEMU exit per byte under HVF"

[[feature]]
id = "shareable"
title = "Images that call shareable images"
status = "todo"
note = "main.rs ponytail: no image activator, so %VRUN-F-SHRIMG"

[[feature]]
id = "fast-files"
title = "A faster file channel than the console UART"
status = "todo"
note = "PRD-0004 open question: if compile times under vrun show it matters"

[[feature]]
id = "sym-elf"
title = "--sym: a symbol-only ELF from the map, for the debugger"
status = "todo"
note = "PRD-0001 stretch goal"

[[needs]]
on = "/host/vms-obj"
why = "Reads the image it loads"

+++

Not a VMS piece: a way to run vaxpunk images without booting vaxpunk.
`vrun` lays out guest RAM with a small EL1 boot stub, page tables that put
the image's sections at their link addresses, a stack with a guard page
and a return page, and starts stock `qemu-system-aarch64` on it, under
TCG or HVF. The stub is built with the cross binutils. The image runs at EL0 and
asks the stub for a console, files and an exit with `SVC`. Faults come
back as VMS-style messages with the nearest symbol from the link map, and
a traceback through standard frames.

It is how the compilers are tested and how the BLISS-64 toolchain runs
on the host: VASM, vasm in BLISS-64, runs under it, and stage 1 of the
BLISS compiler will. It can't run an image that calls a shareable image,
having no image activator.
