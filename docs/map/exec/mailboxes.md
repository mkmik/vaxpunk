+++
title = "Mailboxes"
summary = "Message queues between processes, as MBn: devices"
aka = ["MBX", "MBA", "$CREMBX", "$DELMBX", "MSG$_DELPROC", "termination mailbox", "BUFQUO", "PRMMBX", "TMPMBX", "LNM$TEMPORARY_MAILBOX", "mbdriver.mar"]
kind = "core"
intent = "goal"
research = "done"
milestone = "m0"
refs = ["docs/adr/0019-mailboxes.md", "vms/exec/mbdriver.mar", "docs/prd/0003-multi-user-vms.md"]

[[feature]]
id = "basic"
title = "$CREMBX and $DELMBX; reads that wait, writes done at once; temporary and permanent"
status = "done"
milestone = "m0"
refs = ["docs/adr/0019-mailboxes.md"]

[[feature]]
id = "termination"
title = "MB$SEND: the executive writes a process's termination message"
status = "done"
milestone = "m0"
refs = ["docs/adr/0019-mailboxes.md"]

[[feature]]
id = "writeof"
title = "IO$_WRITEOF, an end of file a read gets as SS$_ENDOFFILE"
status = "done"
milestone = "m1"
note = "PRD-0003 tier 3 #28; came with step 8, for DCL's data lines (dcl/proc.mar)"

[[feature]]
id = "privileges"
title = "PRMMBX and TMPMBX to make one"
status = "done"
milestone = "m1"
note = "PRD-0003 step 1"

[[feature]]
id = "protection"
title = "An owner and a protection mask, $CREMBX's promsk, checked on $ASSIGN"
status = "todo"
milestone = "m1"
needs = ["/exec/uic-protection"]
note = "PRD-0003 tier 1 #1; ADR-0019, mbdriver.mar"

[[feature]]
id = "name-tables"
title = "Names in LNM$TEMPORARY_MAILBOX (the job's) and LNM$PERMANENT_MAILBOX, not SYSNAM's"
status = "todo"
milestone = "m1"
needs = ["/exec/logical-names#job-group"]
note = "PRD-0003 tier 1 #1: a $CREMBX logical name takes SYSNAM until LNM$JOB"

[[feature]]
id = "bufquo"
title = "BUFQUO charged to the creator's byte quota"
status = "todo"
milestone = "m1"
note = "PRD-0003 tier 1 #4 (ADR-0019); with t8y:/exec/quotas#mailbox-bufquo, which needs this component, so no need back"

[[feature]]
id = "attention-asts"
title = "Read and write attention ASTs, IO$_SETMODE with IO$M_READATTN and IO$M_WRTATTN"
status = "todo"
note = "PRD-0003 tier 3 #28: when a program needs them; the job controller may"

[[feature]]
id = "write-wait"
title = "Writes that wait for a reader without IO$M_NOW; resource wait for room"
status = "todo"
needs = ["/exec/scheduler#resource-wait"]
note = "PRD-0003 tier 3 #28; ADR-0019 alternatives"

[[feature]]
id = "assign-mbx"
title = "$ASSIGN's mbxnam: a device's associated mailbox"
status = "todo"
note = "ADR-0019 follow-ups; $DEVICE_SCAN finds mailboxes since ADR-0044"

[[feature]]
id = "channel-checks"
title = "Read-only and write-only channels, IO$M_READERCHECK and IO$M_WRITERCHECK"
status = "todo"

[[feature]]
id = "more-units"
title = "More than 99 mailboxes, named MBAn"
status = "done"
note = "ADR-0044: MBA1 to MBA9999, cloned from a template that isn't a unit, as VMS's"

[[needs]]
on = "/exec/qio"
why = "Its reads and writes are $QIO requests"

[[needs]]
on = "/exec/pool"
why = "Its UCB and each message are pool"

[[needs]]
on = "/exec/logical-names"
why = "Other processes find it by name"

[[needs]]
on = "/exec/privileges"
why = "PRMMBX, TMPMBX, SYSNAM"
+++

A VMS mailbox is a pseudo-device, `MBAn:`, that holds messages between
processes. `$CREMBX` makes one, temporary or permanent, with a logical
name in `LNM$TEMPORARY_MAILBOX` (the job's table) or
`LNM$PERMANENT_MAILBOX`, and assigns a channel; other processes assign
theirs by that name. `$QIO` writes queue a message and, unless
`IO$M_NOW`, wait for a reader; reads take the oldest message or wait.
Attention ASTs tell a process when a message or a reader comes. A
process's termination mailbox gets its `MSG$_DELPROC` message when it
ends, which is how `SPAWN`, the job controller and `OPCOM` learn of it,
and a terminal's associated mailbox gets its unsolicited-input messages.
Mailboxes have an owner and a protection mask, and their messages are
charged to the creator's byte quota.

vaxpunk's are in `vms/exec/mbdriver.mar`
([ADR-0019](../../adr/0019-mailboxes.md)): a UCB from pool per
mailbox, `MBAn`, cloned from a template
([ADR-0044](../../adr/0044-the-io-database.md)), with its messages in
pool. Writes are done at
once, as VMS's with `IO$M_NOW`, and fail with `SS$_MBFULL` when it's
full. Reads wait, and `IO$_WRITEOF` gives a reader an end of file.
Process rundown sends the termination message with `MB$SEND`. Making one
takes `PRMMBX` or `TMPMBX` (PRD-0003 step 1); its name goes in the
system table, so it also takes `SYSNAM`. Missing: the protection mask and
the job's table (PRD-0003 tier 1 #1), `BUFQUO` (tier 1 #4), and attention
ASTs and writes that wait (tier 3 #28).
