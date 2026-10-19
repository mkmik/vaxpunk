+++
title = "Rights identifiers"
summary = "RIGHTSLIST.DAT: identifiers users hold, and each process's rights list"
aka = ["RIGHTSLIST.DAT", "identifier", "rights list", "ADD/IDENTIFIER", "GRANT/IDENTIFIER", "REVOKE/IDENTIFIER", "SET RIGHTS_LIST", "SHOW PROCESS/RIGHTS", "$ASCTOID", "$IDTOASC", "$FIND_HELD", "$FIND_HOLDER", "$GRANTID", "$REVOKID", "INTERACTIVE", "BATCH", "NETWORK", "resource identifier"]
kind = "core"
intent = "goal"
research = "done"
refs = ["docs/adr/0040-acls-in-file-headers-and-rights-identifiers.md", "vms/exec/vecrdb.mar", "vms/rightslist.fdl", "vms/sysexe/authorize.mar", "vms/sysexe/loginout.mar", "docs/prd/0003-multi-user-vms.md"]

[[feature]]
id = "database"
title = "RIGHTSLIST.DAT, an indexed file of identifiers and their holders"
status = "done"
note = "build.rs writes it, SYSTARTUP_VMS.COM copies it to the ramdisk"

[[feature]]
id = "authorize"
title = "AUTHORIZE's ADD/IDENTIFIER, GRANT/IDENTIFIER, REVOKE/IDENTIFIER, SHOW/IDENTIFIER"
status = "done"
note = "authorize.mar: and RENAME, MODIFY/IDENTIFIER, SHOW/RIGHTS"
needs = ["#database", "/utils/authorize"]

[[feature]]
id = "process-rights"
title = "Each process's rights list, built at login from the identifiers its user holds"
status = "done"
note = "LOGINOUT, EXE$LOGIN: PCB$L_RIGHTS; $CREPRC copies it to a process of the same UIC"
needs = ["#database", "/utils/loginout"]

[[feature]]
id = "environmental"
title = "INTERACTIVE, BATCH, NETWORK, LOCAL, DIALUP and REMOTE, by how the process logged in"
status = "wip"
note = "LOGINOUT gives INTERACTIVE and LOCAL or REMOTE; no batch, network or dialup logins yet"
needs = ["#process-rights"]

[[feature]]
id = "system-rights"
title = "The system rights list every process holds: SET RIGHTS_LIST/SYSTEM"
status = "todo"

[[feature]]
id = "services"
title = "$ASCTOID, $IDTOASC, $FIND_HELD, $FIND_HOLDER, $ADD_IDENT, $ADD_HOLDER and the rest"
status = "done"
note = "vecrdb.mar, in the caller's mode; and $MOD_IDENT, $MOD_HOLDER, $FINISH_RDB"
needs = ["#database"]

[[feature]]
id = "dynamic"
title = "$GRANTID and $REVOKID; SET RIGHTS_LIST for a process's own list"
status = "todo"
needs = ["#process-rights"]

[[feature]]
id = "attributes"
title = "Identifier attributes: RESOURCE, DYNAMIC, SUBSYSTEM, NOACCESS, HOLDER_HIDDEN"
status = "todo"
note = "Kept in the records, which AUTHORIZE sets and shows; nothing acts on them"

[[feature]]
id = "named-uics"
title = "UIC identifiers, so [JOE] stands for [200,1]"
status = "done"
note = "$IDTOASC; SHOW SECURITY and DIRECTORY show [SYSTEM]"
needs = ["#database"]

[[feature]]
id = "resource"
title = "Disk space charged to a resource identifier a group of users shares"
status = "todo"
needs = ["#attributes"]
note = "Charged by t8y:/files/disk-quotas"

[[needs]]
on = "/rms/indexed"
why = "RIGHTSLIST.DAT is an indexed file"

[[needs]]
on = "/exec/uic-protection"
why = "UIC identifiers are UICs"

[[needs]]
on = "/exec/processes"
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

vaxpunk has VMS's `RIGHTSLIST.DAT`, which `build.rs` writes with the
environmental identifiers and a UIC identifier for each user, and the
services that read and change it, in their caller's mode, through RMS
([ADR-0040](../../adr/0040-acls-in-file-headers-and-rights-identifiers.md)).
`AUTHORIZE`'s identifier commands use them; `LOGINOUT` gives each
process its rights list, which ACLs (`t8y:/exec/acls`) check and `SHOW
PROCESS/RIGHTS` shows. Missing: `SET RIGHTS_LIST`, system rights, and
anything that acts on identifier attributes.
