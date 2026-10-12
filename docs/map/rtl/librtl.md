+++
title = "LIB$ routines"
summary = "The general-purpose run-time library, LIBRTL.EXE: I/O, signals, strings, VM"
aka = ["LIB$", "LIBRTL", "LIBRTL.EXE", "LIB$PUT_OUTPUT", "LIB$SIGNAL", "LIB$GET_FOREIGN"]
kind = "core"
intent = "goal"
research = "done"
refs = ["docs/adr/0028-shareable-images.md", "docs/adr/0018-set-command-and-foreign-commands.md", "docs/adr/0021-condition-handlers-run-in-the-mode-that-signals.md", "vms/sysexe/librtl/librtl.opt", "vms/sysexe/librtl/putoutput.mar", "vms/sysexe/librtl/getinput.mar", "vms/sysexe/librtl/signal.mar", "vms/sysexe/lib/getforeign.mar"]

[[feature]]
id = "shareable"
title = "LIBRTL.EXE, a shareable image every program but DCL calls"
status = "done"
needs = ["/exec/shareable-images"]
note = "ADR-0028, PRD-0003 tier 3 #20"

[[feature]]
id = "io"
title = "LIB$PUT_OUTPUT and LIB$GET_INPUT"
status = "done"
note = "ADR-0028"

[[feature]]
id = "signals"
title = "LIB$SIGNAL, LIB$STOP, LIB$ESTABLISH, LIB$REVERT, LIB$SIG_TO_RET"
status = "done"
needs = ["/exec/condition-handling"]
note = "ADR-0021, ADR-0028"

[[feature]]
id = "get-foreign"
title = "LIB$GET_FOREIGN, linked into each image, not in LIBRTL.EXE"
status = "done"
note = "ADR-0018; ADR-0028 keeps it out until the CLI$ routines ask DCL"

[[feature]]
id = "get-input-padding"
title = "LIB$GET_INPUT blank-pads a fixed-length string"
status = "todo"
note = "PRD-0003 tier 3 #36"

[[feature]]
id = "installed"
title = "Installed shared, its pages shared by every process"
status = "todo"
needs = ["/exec/global-sections", "/exec/installed-images"]
note = "PRD-0003 tier 3 #20"

[[feature]]
id = "find-image-symbol"
title = "LIB$FIND_IMAGE_SYMBOL: call a shareable image found at run time"
status = "todo"
note = "ADR-0028 follow-up"

[[feature]]
id = "strings-time"
title = "String and time routines: LIB$SYS_FAO, LIB$CVT_DTB, LIB$DATE_TIME, LIB$GET_SYMBOL, ..."
status = "todo"
note = "ADR-0028 follow-up: as programs need them"

[[feature]]
id = "vm"
title = "Virtual memory zones: LIB$GET_VM, LIB$FREE_VM, LIB$CREATE_VM_ZONE"
status = "todo"

[[feature]]
id = "processes"
title = "LIB$SPAWN, LIB$DO_COMMAND, LIB$SET_SYMBOL: talking to DCL"
status = "todo"
note = "LIB$SPAWN needs DCL's SPAWN (t8y:/dcl/spawn)"

[[feature]]
id = "files"
title = "LIB$FIND_FILE, LIB$RENAME_FILE, LIB$DELETE_FILE, LIB$CREATE_DIR"
status = "todo"
note = "PRD-0007's WASI layer uses them"

[[feature]]
id = "data-vector"
title = "DATA entries in the symbol vector"
status = "todo"
note = "ADR-0028 follow-up; the C RTL's stdout and errno need them"

[[needs]]
on = "/exec/calling-standard"
why = "Every routine is called by the standard, from any language"

[[needs]]
on = "/exec/qio"
why = "LIB$PUT_OUTPUT and LIB$GET_INPUT are $QIOW on SYS$OUTPUT and SYS$INPUT"

[[needs]]
on = "/host/vlink"
why = "vlink makes it with /SHAREABLE and its SYMBOL_VECTOR"
+++

`LIB$` is the general-purpose part of VMS's run-time library, in the
shareable image `LIBRTL.EXE`: about 200 routines for terminal and
command-line I/O (`LIB$PUT_OUTPUT`, `LIB$GET_INPUT`,
`LIB$GET_FOREIGN`), condition handling (`LIB$SIGNAL`, `LIB$ESTABLISH`),
virtual memory zones, string and date conversion, logical names and
symbols, spawning subprocesses, files, bit fields and queues. Programs
in every language call them, and an image linked against the symbol
vector keeps working with a newer `LIBRTL.EXE`.

vaxpunk's `LIBRTL.EXE`, from `vms/sysexe/librtl/`, holds
`LIB$PUT_OUTPUT`, `LIB$GET_INPUT` and the condition handling routines,
with a symbol vector in `librtl.opt`
([ADR-0028](../../adr/0028-shareable-images.md)). The image
activator maps a copy into each process, since global sections aren't
there yet. `LIB$GET_FOREIGN` is linked into each image, next to the
`CLI$` parser it calls. The rest comes as programs need it.
