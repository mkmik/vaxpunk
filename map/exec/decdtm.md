+++
title = "DECdtm"
summary = "Distributed transactions: two-phase commit across resource managers and nodes"
aka = ["DECdtm services", "$START_TRANS", "$END_TRANS", "$ABORT_TRANS", "$START_TRANSW", "$END_TRANSW", "$GETDTI", "$SETDTI", "$DECLARE_RM", "$JOIN_RM", "$ACK_EVENT", "$ADD_BRANCH", "$START_BRANCH", "TID", "TP_SERVER", "LMCP", "SYSTEM$node.LM$JOURNAL", "SYS$JOURNAL", "XA gateway", "two-phase commit"]
kind = "core"
intent = "aspiration"
research = "outline"

[[feature]]
id = "services"
title = "$START_TRANS, $END_TRANS, $ABORT_TRANS and their W forms on one node"
status = "todo"
needs = [":log"]

[[feature]]
id = "log"
title = "The transaction log, SYSTEM$node.LM$JOURNAL, made and managed with LMCP"
status = "todo"

[[feature]]
id = "tp-server"
title = "TP_SERVER, the process that writes the log and recovers transactions after a crash"
status = "todo"
needs = [":log"]

[[feature]]
id = "resource-managers"
title = "The resource manager interface: $DECLARE_RM, $JOIN_RM, $ACK_EVENT, the prepare and commit events"
status = "todo"
needs = [":services"]

[[feature]]
id = "rms-recovery-units"
title = "RMS recovery unit journaling as a resource manager"
status = "todo"
needs = [":resource-managers"]
note = "//rms/journaling's recovery units are DECdtm transactions"

[[feature]]
id = "distributed"
title = "Branches on other nodes: $ADD_BRANCH, $START_BRANCH, commit across the network"
status = "todo"
needs = [":resource-managers"]

[[feature]]
id = "inquiry"
title = "$GETDTI and $SETDTI; LMCP's SHOW TRANSACTION and recovery of a blocked transaction"
status = "todo"
needs = [":tp-server"]

[[feature]]
id = "xa"
title = "The XA gateway: DECdtm with X/Open XA resource managers and transaction managers"
status = "todo"
needs = [":resource-managers"]

[[needs]]
on = "//exec/lock-manager"
why = "Resource managers lock what a transaction touches until it ends"

[[needs]]
on = "//rms/sequential"
why = "The transaction log is a file it appends to and reads back at recovery"

[[needs]]
on = "//exec/cluster"
why = "In a cluster any node's TP_SERVER can recover another's transactions"
soft = true

[[needs]]
fuzzy = "A transport between nodes' transaction managers: DECnet on VMS, TCP/IP here"
why = "Two-phase commit across nodes sends prepare and commit messages"

[[needs]]
fuzzy = "The transaction log's record format and the recovery protocol"
why = "Recovery after a crash must find every transaction's outcome"
+++

DECdtm services, part of OpenVMS since V5.4, coordinate transactions
that span several resource managers, such as Rdb databases, RMS
journaled files, DBMS and ACMS, and several nodes. A program starts a
transaction with `$START_TRANS`, gets a transaction ID, passes it to
each resource manager it calls, and ends it with `$END_TRANS` or
`$ABORT_TRANS`. DECdtm then runs two-phase commit: it asks every
resource manager to prepare, writes the decision to its transaction
log, and tells them to commit or roll back. `TP_SERVER` writes the log
and, after a crash, finds the outcome of transactions left in doubt;
`LMCP`, the Log Manager Control Program, creates and examines the logs.
Resource managers join with `$DECLARE_RM` and `$JOIN_RM`. An XA
gateway lets DECdtm work with X/Open XA software such as Oracle.

vaxpunk has none, and no resource manager to coordinate. It would
matter for the signature application, an inventory system on ACMS and
RMS indexed files (`//layered/home-inventory`): ACMS tasks that update
RMS files atomically do so through RMS recovery units, which are
DECdtm transactions.
