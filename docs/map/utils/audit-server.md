+++
title = "Audit server"
summary = "AUDIT_SERVER: security events into SECURITY.AUDIT$JOURNAL; ANALYZE/AUDIT"
aka = ["AUDIT_SERVER", "AUDIT$SERVER", "SECURITY.AUDIT$JOURNAL", "SET AUDIT", "SHOW AUDIT", "ANALYZE/AUDIT", "audit listener", "security archive file"]
kind = "core"
intent = "aspiration"
research = "outline"
refs = ["docs/prd/0003-multi-user-vms.md", "https://space.physics.uiowa.edu/pi/docs/vms83/opsys/vmsos831h1/aa_q2hlg_te/ch09s04.html"]

[[feature]]
id = "journal"
title = "The AUDIT_SERVER process writes events to SYS$MANAGER:SECURITY.AUDIT$JOURNAL"
status = "todo"
needs = ["/exec/mailboxes", "/rms/sequential"]

[[feature]]
id = "set-audit"
title = "SET AUDIT /ENABLE, /DISABLE, /SERVER=NEW_LOG; SHOW AUDIT"
status = "todo"
needs = ["#journal"]

[[feature]]
id = "analyze"
title = "ANALYZE/AUDIT: reports from the journal, selected by event, user and time"
status = "todo"
needs = ["#journal"]

[[feature]]
id = "listener"
title = "An audit listener mailbox and a remote archive file"
status = "todo"
needs = ["#journal"]

[[needs]]
on = "/exec/processes"
why = "A detached process started at boot"

[[needs]]
on = "/exec/mailboxes"
why = "The executive sends it audit records through a mailbox"

[[needs]]
on = "/rms/sequential"
why = "The journal is a file of binary records"
+++

When security auditing is on, the executive and the utilities report
security events (logins and failures, breakin attempts, object access
under an ACL, privilege use, `AUTHORIZE` changes) to the audit server,
`AUDIT_SERVER`, a process started at boot. It writes each to the
cluster's binary audit journal, `SECURITY.AUDIT$JOURNAL`, sends alarms
to terminals enabled for the `SECURITY` class through OPCOM, and can
copy events to a listener mailbox or a remote archive file. `SET AUDIT`
chooses which events are journaled or raise alarms and starts a new
journal; `ANALYZE/AUDIT` prints and selects them.

vaxpunk has none. Security auditing and `SET AUDIT` are a non-goal of
PRD-0003; they are out of Milestone 1, with ACLs, and the executive's
side is `t8y:/exec/security-audit`.
