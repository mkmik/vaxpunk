+++
title = "RMS core"
summary = "FAB, RAB, NAM and XAB; $OPEN, $CREATE, $CLOSE, $ERASE; file names and defaults"
aka = ["RMS", "FAB", "RAB", "NAM", "NAML", "XAB", "$OPEN", "$CREATE", "$CONNECT", "$CLOSE", "$ERASE", "$DISPLAY", "$RENAME", "$SETDDIR", "IFAB", "RMS$_", "rms.mar"]
kind = "core"
intent = "goal"
research = "done"
milestone = "m0"
refs = ["docs/adr/0007-system-disk-files-11-and-rms.md", "docs/adr/0009-ramdisk-writable-files-11.md", "docs/prd/0008-rms-record-and-indexed-files.md", "docs/prd/0003-multi-user-vms.md", "docs/design/0002-executive-processes.md", "vms/exec/rms.mar", "vms/exec/rmsxab.mar", "vms/sysexe/rmstest.mar", "boot/tests/rms.rs", "https://wasd.vsm.com.au/htbin/helpgate/HELP/RMS"]

[[feature]]
id = "basic"
title = "FAB, RAB and NAM; $OPEN, $CREATE, $CONNECT, $DISCONNECT, $CLOSE, $ERASE"
status = "done"
milestone = "m0"
refs = ["docs/adr/0007-system-disk-files-11-and-rms.md", "docs/adr/0009-ramdisk-writable-files-11.md"]

[[feature]]
id = "names"
title = "Specifications with defaults, SYS$DISK, the default directory, [-] and [.SUB]; $SETDDIR"
status = "done"
milestone = "m0"
note = "DESIGN-0002 Files"

[[feature]]
id = "logical-names"
title = "Logical names and search lists in specifications, as OpenVMS treats them"
status = "done"
note = "PRs #119, #121, #122; DESIGN-0002 Files"

[[feature]]
id = "rundown"
title = "An image's files closed when it exits, as its channels are deassigned"
status = "done"
milestone = "m0"
note = "ADR-0007 decision 5; RMS$RUNDOWN"

[[feature]]
id = "protection"
title = "Owner and protection checked on $OPEN, $CREATE and $ERASE; XABPRO"
status = "done"
milestone = "m1"
note = "PRD-0003 step 2"

[[feature]]
id = "xabs"
title = "XABFHC, XABDAT, XABRDT, XABALL, XABKEY, XABSUM on $OPEN, $CREATE and $DISPLAY"
status = "done"
note = "PRD-0008 step 3; rmsxab.mar"

[[feature]]
id = "block-io"
title = "Block I/O on any organization: $READ, $WRITE, $SPACE"
status = "done"
note = "PRD-0008 step 3, PR #100 for BACKUP"

[[feature]]
id = "messages"
title = "VMS's RMS$_ condition values, with their message texts"
status = "done"
note = "PRD-0008: $RMSDEF in starlet.mlb"

[[feature]]
id = "rename"
title = "$RENAME"
status = "todo"
note = "PRD-0007 M6 needs it; t8y:/utils/rename"

[[feature]]
id = "enter-remove"
title = "$ENTER and $REMOVE: directory entries without the file"
status = "todo"

[[feature]]
id = "extend"
title = "$EXTEND, and the default extension quantity"
status = "todo"
note = "rmsseq.mar extends by 8 blocks"

[[feature]]
id = "bro"
title = "Mixed block and record access, FAB$M_BRO, on relative and indexed files"
status = "todo"
note = "rms.mar ponytail: BRO on them is BIO"

[[feature]]
id = "devices"
title = "$OPEN of terminals, mailboxes and NL:; records as $QIO reads and writes"
status = "todo"
needs = ["/drivers/terminal", "/exec/mailboxes"]
note = "FIL$SELECT knows disks only; LIB$PUT_OUTPUT and the C library write SYS$OUTPUT with $QIO"

[[feature]]
id = "related-names"
title = "Related file names, NAM$L_RLF, for an output specification's defaults"
status = "todo"
note = "PRD-0003 tier 3 #24: COPY and RENAME with wildcards"

[[feature]]
id = "concealed"
title = "Concealed devices and rooted directories, SYS$SYSROOT:[SYSEXE] as [SYS0.SYSEXE]"
status = "todo"
needs = ["/exec/logical-names"]
note = "PRD-0003 tier 3 #25; rms.mar ponytail"

[[feature]]
id = "xqp-modify"
title = "SET PROTECTION and DIRECTORY/PROTECTION through the XQP, without read access"
status = "todo"
needs = ["/files/ods2#qio-interface"]
note = "PRD-0003 tier 3 #23"

[[feature]]
id = "exec-mode"
title = "RMS in executive mode, its buffers and IFABs in the process's P1"
status = "todo"
needs = ["/platform/access-mode-threads"]
note = "PRD-0003 tier 3 #23; PRD-0008 open question on buffers"

[[feature]]
id = "naml"
title = "The NAML block and long specifications, for ODS-5 names"
status = "todo"
needs = ["/files/ods5"]

[[feature]]
id = "rab64"
title = "RAB64: buffers in 64-bit space"
status = "todo"
needs = ["/exec/memory"]

[[needs]]
on = "/files/ods2"
why = "Calls the XQP's FIL$ routines directly for headers, maps and directories"

[[needs]]
on = "/files/volumes"
why = "A specification's device selects a mounted volume's VCB"

[[needs]]
on = "/files/fs-locking"
why = "Every service holds the file system's lock"

[[needs]]
on = "/exec/system-services"
why = "Each RMS service is a system service in the vector"

[[needs]]
on = "/exec/logical-names"
why = "Devices and search lists in specifications are logical names"

[[needs]]
on = "/exec/pool"
why = "Each open file is an IFAB in nonpaged pool, its buffers too"

[[needs]]
on = "/exec/uic-protection"
why = "FIL$CHKPRO on every open, create and erase"

[[needs]]
on = "/exec/processes"
why = "Open files belong to the PCB and close at rundown"

[[needs]]
on = "/host/ods"
why = "The layouts come from crosstools/ods/docs; the boot test checks files with it"
soft = true
+++

RMS gives VMS programs one interface to files: a file access block
(FAB) names a file and says how to open it, a record access block (RAB)
reads and writes its records, a name block (NAM, or NAML for ODS-5's long
names) receives the expanded and resultant specifications, and extended
attribute blocks (XABs) chained from the FAB carry the rest: dates,
protection, allocation, keys. A specification, `node::dev:[dir]name.type;ver`,
gets what it leaves out from a default specification, a related file,
`SYS$DISK` and the process's default directory, after logical names and
search lists are translated. The services return `RMS$_` condition values
in `FAB$L_STS` or `RAB$L_STS`. RMS runs in executive mode, in the
process, with its structures in P1, and calls the XQP with `$QIO`.

vaxpunk's RMS is a set of system services in the executive, in kernel
mode, holding the file system's lock and calling `f11.mar`'s routines
directly ([ADR-0007](../../adr/0007-system-disk-files-11-and-rms.md)).
`rms.mar` has the FAB, RAB and NAM services, file specifications with
logical names, search lists and relative directories, and the protection
checks; it dispatches record services to the organization's module, and
`rmsxab.mar` fills the XABs. `RMSTEST` runs the same MACRO-32 on OpenVMS
Alpha and on vaxpunk and must print the same
([PRD-0008](../../prd/0008-rms-record-and-indexed-files.md)).
Missing: executive mode, `$RENAME`, `$ENTER`, `$REMOVE`, `$EXTEND`,
non-disk devices, concealed devices, ASTs (`t8y:/rms/async`) and sharing
(`t8y:/rms/sharing`).
