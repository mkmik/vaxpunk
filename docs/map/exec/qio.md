+++
title = "$QIO, IRPs and drivers"
summary = "Channels, $QIO, I/O request packets, UCBs and the driver interface"
aka = ["$QIO", "$QIOW", "$ASSIGN", "$DASSGN", "$CANCEL", "$SYNCH", "IRP", "UCB", "DDB", "CRB", "IDB", "DDT", "FDT", "IOSB", "IOC$REQCOM", "IOC$POST", "I/O database", "IOC$GL_DEVLIST", "IOC$SEARCHDEV", "IOC$CLONE_UCB", "template device", "BIOLM", "DIOLM", "BYTLM", "Fast I/O", "Fast Path", "qio.mar"]
kind = "core"
intent = "goal"
research = "done"
milestone = "m0"
refs = ["docs/adr/0013-qio-irps-and-drivers.md", "docs/adr/0015-file-system-io-through-the-disk-driver.md", "docs/adr/0016-tcpip-component-and-bga0.md", "docs/adr/0027-terminals-are-ucbs-and-telnet-is-in-the-driver.md", "docs/adr/0044-the-io-database.md", "docs/design/0002-executive-processes.md", "vms/exec/qio.mar", "docs/prd/0003-multi-user-vms.md"]

[[feature]]
id = "basic"
title = "$ASSIGN, $DASSGN, $QIO, $QIOW: a UCB per device, an FDT routine, an IRP"
status = "done"
milestone = "m0"
refs = ["docs/adr/0013-qio-irps-and-drivers.md"]

[[feature]]
id = "completion"
title = "Completion as a kernel mode AST: data, IOSB, event flag, then the caller's AST"
status = "done"
milestone = "m0"
refs = ["docs/adr/0013-qio-irps-and-drivers.md"]

[[feature]]
id = "device-names"
title = "Device names through logical names, as $ASSIGN translates them"
status = "done"
milestone = "m0"

[[feature]]
id = "cancel"
title = "$CANCEL, and I/O cancelled at rundown and $DASSGN"
status = "done"
milestone = "m0"
refs = ["docs/adr/0014-ctrlc-ctrly-asts.md"]

[[feature]]
id = "cloned-units"
title = "Units cloned into pool from a template: network sockets, mailboxes, TELNET, virtual and pseudo terminals"
status = "done"
milestone = "m0"
refs = ["docs/adr/0016-tcpip-component-and-bga0.md", "docs/adr/0019-mailboxes.md", "docs/adr/0027-terminals-are-ucbs-and-telnet-is-in-the-driver.md", "docs/adr/0044-the-io-database.md"]
note = "ADR-0044: IOC$CLONE_UCB numbers them 1 to 9999, the first free past the template's seed, as VMS's; $ASSIGN clones BG0:'s"

[[feature]]
id = "probe-post"
title = "IOC$POST probes the caller's buffer and IOSB, or the pages are locked"
status = "todo"
milestone = "m1"
note = "PRD-0003 tier 2 #10: deleting the pages under a pending read takes the system down"

[[feature]]
id = "cancel-wait"
title = "IOC$CANCEL waits for rundown's cancels to finish"
status = "todo"
milestone = "m1"
note = "PRD-0003 tier 2 #18; needed once a driver can't stop at once"

[[feature]]
id = "io-database"
title = "Every device a UCB on one list, the network's and the disks' too"
status = "done"
milestone = "m1"
refs = ["docs/adr/0044-the-io-database.md"]
note = "ADR-0044: IOC$GL_DEVLIST's DDBs and their units' UCBs, a DDT per driver, VMS's IOC$SEARCHDEV and IOC$SCAN_IODB"

[[feature]]
id = "quotas"
title = "BIOLM, DIOLM and BYTLM charged and checked; I/O counts kept"
status = "done"
milestone = "m1"
note = "ADR-0038; the limits and the charging are t8y:/exec/quotas#pool-io, which needs this component, so no need back"

[[feature]]
id = "allocate"
title = "$ALLOC and $DALLOC: a device allocated to one process"
status = "todo"
note = "A private mount allocates its disk already, in VCB$L_PID (t8y:/files/volumes#private), which $ASSIGN checks; UCB$L_PID would hold it here"

[[feature]]
id = "driver-structures"
title = "VMS's CRB, IDB and FDT tables, and drivers loaded at run time"
status = "todo"
note = "ADR-0044 has the DDB and the DDT, whose FDT is one routine per driver, which checks the functions itself"

[[feature]]
id = "fast-io"
title = "Fast I/O and Fast Path: $IO_SETUP, $IO_PERFORM, buffer objects"
status = "todo"

[[needs]]
on = "/exec/pool"
why = "IRPs and their buffers come from pool"

[[needs]]
on = "/exec/asts"
why = "Completion is a kernel mode AST, then the caller's"

[[needs]]
on = "/exec/event-flags"
why = "Each request sets an event flag; $QIOW waits for it"

[[needs]]
on = "/exec/ipl-sync"
why = "FDT routines and completion at IPL$_SYNCH; drivers fork"

[[needs]]
on = "/exec/logical-names"
why = "$ASSIGN translates the device name"

[[needs]]
on = "/exec/memory"
why = "Probes the caller's buffer and IOSB"
+++

All VMS I/O goes through `$QIO`. A process assigns a channel to a device
(`$ASSIGN`), and each request on it is an I/O request packet (IRP) that
the driver's FDT routines check and its start I/O routine works through,
one at a time per unit, at interrupt and fork level. `IOC$REQCOM` puts a
finished IRP on the I/O post queue, and a kernel mode AST in the process
copies a buffered read's data, writes the I/O status block, sets the
event flag and queues the caller's AST. The I/O database describes each
device: a device data block (DDB) per controller type, a unit control
block (UCB) per unit, the channel request and interrupt dispatch blocks,
and the driver's dispatch table. RMS, the XQP, TCP/IP and DCL are all
`$QIO` callers. Alpha V7 added Fast I/O and Fast Path for heavy disk
traffic.

vaxpunk's is `vms/exec/qio.mar`
([ADR-0013](../../adr/0013-qio-irps-and-drivers.md)). The devices are
VMS's I/O database
([ADR-0044](../../adr/0044-the-io-database.md)): `IOC$GL_DEVLIST`'s
DDBs, each with its units' UCBs by number and its driver's DDT, which
`$ASSIGN`, `$GETDVI` and `$DEVICE_SCAN` search as VMS's do. The driver's
FDT routine makes the IRP and does the I/O or queues it, and completion
is a kernel mode AST, `IOC$POST`, which turns the IRP into the caller's
AST. There is no I/O post queue, since drivers complete at
`IPL$_SYNCH`. The drivers are the terminals', the disks' (whose start
I/O routine the file system calls too,
[ADR-0015](../../adr/0015-file-system-io-through-the-disk-driver.md)),
the network's and the mailboxes'; network units, mailboxes and TELNET,
virtual and pseudo terminals are units cloned into pool from a
template. Missing: probing in `IOC$POST` (PRD-0003 tier 2 #10), CRBs,
IDBs, FDT tables and loadable drivers.
