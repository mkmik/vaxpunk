+++
title = "Rights identifiers"
summary = "RIGHTSLIST.DAT: identifiers users hold, and each process's rights list"
aka = ["RIGHTSLIST.DAT", "identifier", "rights list", "ADD/IDENTIFIER", "GRANT/IDENTIFIER", "REVOKE/IDENTIFIER", "SET RIGHTS_LIST", "SHOW PROCESS/RIGHTS", "$ASCTOID", "$IDTOASC", "$FIND_HELD", "$FIND_HOLDER", "$GRANTID", "$REVOKID", "INTERACTIVE", "BATCH", "NETWORK", "resource identifier"]
kind = "core"
intent = "aspiration"
research = "outline"
refs = ["docs/prd/0003-multi-user-vms.md", "vms/sysexe/authorize.mar"]

[[feature]]
id = "database"
title = "RIGHTSLIST.DAT, an indexed file of identifiers and their holders"
status = "todo"

[[feature]]
id = "authorize"
title = "AUTHORIZE's ADD/IDENTIFIER, GRANT/IDENTIFIER, REVOKE/IDENTIFIER, SHOW/IDENTIFIER"
status = "todo"
needs = [":database", "//utils/authorize"]
note = "authorize.mar ponytail: no rights database"

[[feature]]
id = "process-rights"
title = "Each process's rights list, built at login from the identifiers its user holds"
status = "todo"
needs = [":database", "//utils/loginout"]
note = "PRD-0003 gives a process only its UIC"

[[feature]]
id = "environmental"
title = "INTERACTIVE, BATCH, NETWORK, LOCAL, DIALUP and REMOTE, by how the process logged in"
status = "todo"
needs = [":process-rights"]

[[feature]]
id = "system-rights"
title = "The system rights list every process holds: SET RIGHTS_LIST/SYSTEM"
status = "todo"

[[feature]]
id = "services"
title = "$ASCTOID, $IDTOASC, $FIND_HELD, $FIND_HOLDER, $ADD_IDENT, $GRANTID, $REVOKID"
status = "todo"
needs = [":database"]

[[feature]]
id = "attributes"
title = "Identifier attributes: RESOURCE, DYNAMIC, SUBSYSTEM, NOACCESS, HOLDER_HIDDEN"
status = "todo"

[[feature]]
id = "named-uics"
title = "UIC identifiers, so [JOE] stands for [200,1]"
status = "todo"
needs = [":database"]

[[feature]]
id = "resource"
title = "Disk space charged to a resource identifier a group of users shares"
status = "todo"
needs = [":attributes"]
note = "Charged by //files/disk-quotas"

[[needs]]
on = "//rms/indexed"
why = "RIGHTSLIST.DAT is an indexed file"

[[needs]]
on = "//exec/uic-protection"
why = "UIC identifiers are UICs"

[[needs]]
on = "//exec/processes"
why = "A process's rights list hangs off its PCB, or its persona"
soft = true
+++

The rights database, `SYS$SYSTEM:RIGHTSLIST.DAT`, is an indexed file of
identifiers and of who holds them. Each user has a UIC identifier, the
username standing for the UIC; a site adds general identifiers, such as
`PAYROLL`, with `AUTHORIZE ADD/IDENTIFIER` and grants them with
`GRANT/IDENTIFIER`. VMS grants environmental identifiers by how one
logged in: `INTERACTIVE`, `BATCH`, `NETWORK`, `LOCAL`, `DIALUP`,
`REMOTE`. At login `LOGINOUT` builds the process's rights list from
them; there is also a system rights list every process holds. ACLs
grant access to identifiers rather than UICs, so one file can be shared
with any set of users. Attributes change what holding one means: a
resource identifier lets its holders charge disk space to it, a dynamic
one can be turned off and on with `SET RIGHTS_LIST`, a subsystem one
marks a protected subsystem.

vaxpunk has none. PRD-0003 gives a process "a rights list of its UIC
alone", which is just its UIC, and `AUTHORIZE` has no identifier
commands. Nothing needs identifiers until ACLs (`//exec/acls`) or a
server that maps foreign users onto VMS ones (`//net/cifs`).
