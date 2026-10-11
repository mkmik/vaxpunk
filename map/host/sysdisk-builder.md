+++
title = "System disk builder"
summary = "vms/build.rs: compiles and links every image and writes sysdisk.img"
aka = ["vms/build.rs", "vms/c.rs", "sysdisk.img", "EXEC.EXE", "SYS.STB"]
kind = "tool"
intent = "goal"
research = "done"
milestone = "m0"
refs = ["vms/build.rs", "vms/c.rs", "crosstools/README.md", "docs/adr/0007-system-disk-files-11-and-rms.md", "docs/adr/0028-shareable-images.md", "docs/adr/0032-c-run-time-library-and-ssl3-on-mbed-tls.md", "docs/prd/0007-wasmrun.md"]

[[feature]]
id = "images"
title = "EXEC.EXE in S0 and an image per sysexe program, with SYS.STB, on an ODS-2 volume"
status = "done"
milestone = "m0"

[[feature]]
id = "tables"
title = "DCL$TABLES and each utility's command table from CLD"
status = "done"
milestone = "m0"
needs = ["//host/vcdu"]
note = "ADR-0017"

[[feature]]
id = "bliss"
title = "BLISS-64 modules in the executive and programs, failing on any warning"
status = "done"
needs = ["//host/vbliss"]
note = "PRD-0004 step 11; TPU and EVE, PRD-0006"

[[feature]]
id = "shareable"
title = "LIBRTL.EXE as a shareable image from librtl.opt"
status = "done"
note = "ADR-0028"

[[feature]]
id = "files"
title = "SYSUAF.DAT loaded from FDL, startup procedures, SYSMGR text files"
status = "done"
note = "ADR-0029"

[[feature]]
id = "c"
title = "The C run-time library, SSL3 and Mbed TLS, CDEMO and the C programs"
status = "done"
needs = ["//host/velf", "//host/cross-gcc", "//host/vlib"]
note = "vms/c.rs, ADR-0032"

[[feature]]
id = "multi-module"
title = "An image of many modules of its own, as WASMRUN's"
status = "todo"
note = "PRD-0007 Building it and M0: one image per sysexe/*.mar, DCL a special case"

[[needs]]
on = "//host/vmacro"
why = "Compiles the executive, drivers and utilities"

[[needs]]
on = "//host/vasm"
why = "Assembles the .m64 ARM64 sources"

[[needs]]
on = "//host/vlink"
why = "Links every image"

[[needs]]
on = "//host/ods"
why = "ods-image writes the volume"
+++

A VMS system disk is made by installing VMS from its distribution
media, then `SYSGEN`, `AUTOGEN` and the startup procedures. vaxpunk's is
made on the host by `vms/build.rs`, the build script of the `vms` crate:
it compiles every MACRO-32, BLISS-64, ARM64 and C source under `vms/`,
links `EXEC.EXE` into S0 and each program against `LIBRTL.EXE` and
`SYS.STB` (the executive's symbols), and writes them with the startup
files and `SYSUAF.DAT` to `sysdisk.img`, an ODS-2 volume, through
`ods-image`.

It is the one place that knows how the system is put together. The
boot crate copies its output to `out/`. WASMRUN will need it to link an
image from many modules of its own.
