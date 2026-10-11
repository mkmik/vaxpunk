+++
title = "Security auditing"
summary = "Security events written to an audit journal and raised as alarms"
aka = ["SET AUDIT", "SHOW AUDIT", "AUDIT_SERVER", "SECURITY.AUDIT$JOURNAL", "ANALYZE/AUDIT", "security alarm", "security operator", "$AUDIT_EVENT", "$CHECK_PRIVILEGE", "NSA$", "VMS$AUDIT_SERVER.DAT", "AUDIT privilege", "SECURITY privilege"]
kind = "core"
intent = "aspiration"
research = "outline"
refs = ["docs/prd/0003-multi-user-vms.md"]

[[feature]]
id = "events"
title = "The executive raises events: logins, failures, break-ins, privilege use, file access, mounts"
status = "todo"

[[feature]]
id = "journal"
title = "The audit server writes them to SECURITY.AUDIT$JOURNAL"
status = "todo"
needs = ["#events", "/utils/audit-server"]

[[feature]]
id = "alarms"
title = "Alarms on security operators' terminals through OPCOM"
status = "todo"
needs = ["#events", "/utils/opcom"]

[[feature]]
id = "set-audit"
title = "SET AUDIT and SHOW AUDIT choose the event classes for alarms and for the journal"
status = "todo"
needs = ["#events"]

[[feature]]
id = "analyze"
title = "ANALYZE/AUDIT selects and reports records from the journal"
status = "todo"
needs = ["#journal"]

[[feature]]
id = "services"
title = "$AUDIT_EVENT and $CHECK_PRIVILEGE for programs that make their own checks"
status = "todo"
needs = ["#events"]

[[feature]]
id = "breakin"
title = "Break-in and login failure alarms"
status = "todo"
needs = ["#alarms", "/exec/intrusion-detection"]

[[needs]]
on = "/exec/privileges"
why = "AUDIT and SECURITY; privilege use is an event class"

[[needs]]
on = "/exec/acls"
why = "Alarm and audit ACEs ask for events on one object"
soft = true

[[needs]]
fuzzy = "The audit record format (NSA$) and the list of event classes"
why = "ANALYZE/AUDIT and other systems read the records"
+++

Security auditing records security-relevant events. The executive and
the privileged images notice an event, such as a login, a login
failure, a break-in, a privilege used, a file opened or deleted, a
volume mounted, a system parameter changed, or an `AUTHORIZE` change,
and pass a record to `AUDIT_SERVER`, a process that writes it to the
journal, `SYS$MANAGER:SECURITY.AUDIT$JOURNAL`, and sends alarms through
OPCOM to terminals enabled with `REPLY/ENABLE=SECURITY`. `SET AUDIT`
says which event classes go where; `ANALYZE/AUDIT` reads the journal.
Programs that make their own access decisions report them with
`$AUDIT_EVENT` and `$CHECK_PRIVILEGE`. The audit trail is part of what
earned VMS its US government C2 rating.

vaxpunk has none: auditing and `SET AUDIT` are non-goals of PRD-0003.
They wait on OPCOM, an audit server, and something worth auditing
beyond logins.
