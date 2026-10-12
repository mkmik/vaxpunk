+++
title = "LOGINOUT"
summary = "Logs a terminal's process in from SYSUAF.DAT, and LOGOUT out"
aka = ["LOGINOUT", "LOGINOUT.EXE", "Username:", "LOGOUT", "SYSALF.DAT", "Automatic Login Facility", "SYS$WELCOME", "SYS$ANNOUNCE", "LGI$"]
kind = "core"
intent = "goal"
research = "done"
milestone = "m1"
refs = ["vms/sysexe/loginout.mar", "vms/exec/hashpwd.mar", "docs/adr/0029-loginout-and-sysuaf.md", "docs/prd/0003-multi-user-vms.md", "docs/design/0002-executive-processes.md", "https://docs.vmssoftware.com/vsi-openvms-system-manager-s-manual-volume-1-essentials/"]

[[feature]]
id = "console"
title = "Username: and Password: on the console, checked against SYSUAF.DAT"
status = "done"
milestone = "m1"
note = "PRD-0003 step 4, ADR-0029"

[[feature]]
id = "telnet"
title = "Logins on TELNETD's TNAn: terminals, over SET HOST"
status = "done"
milestone = "m1"
note = "ADR-0029 decision 3; TELNETD starts it (t8y:/net/telnet)"

[[feature]]
id = "user-process"
title = "EXE$LOGIN: the user's UIC, privileges, priority and command interpreter"
status = "done"
milestone = "m1"
note = "ADR-0029 decision 4"

[[feature]]
id = "failures"
title = "Three failures end it; only a wrong password counts, reported at the next login"
status = "done"
milestone = "m1"

[[feature]]
id = "logout"
title = "LOGOUT: logged out at, and the process deleted"
status = "done"
milestone = "m1"
note = "ADR-0029 decision 5"

[[feature]]
id = "startup-process"
title = "STARTUP runs SYSTARTUP_VMS.COM before anyone logs in"
status = "done"
milestone = "m1"
note = "PRD-0003 tier 1 #2"

[[feature]]
id = "alf"
title = "Automatic Login Facility: a terminal logged in at once as a user"
status = "done"
note = "PR #139; SYSALF.DAT is text, not the indexed file SYSMAN writes"

[[feature]]
id = "quotas"
title = "The UAF's quotas given to the process"
status = "todo"
milestone = "m1"
needs = ["/exec/quotas#login"]
note = "PRD-0003 tier 1 #4, step 5; loginout.mar ponytail"

[[feature]]
id = "process-protection"
title = "The process's default file protection from the UAF"
status = "todo"
milestone = "m1"
needs = ["/files/ods2#process-default-protection"]
note = "PRD-0003 tier 1 #8"

[[feature]]
id = "job-tables"
title = "LNM$JOB and LNM$GROUP for each logged-in job; user-mode logical names"
status = "todo"
milestone = "m1"
needs = ["/exec/logical-names#job-group"]
note = "PRD-0003 tier 1 #1 and tier 3 #25"

[[feature]]
id = "login-at-key"
title = "The console asks for a username at a key, not at once"
status = "todo"
milestone = "m1"
needs = ["/drivers/terminal#login-at-key"]
note = "PRD-0003 tier 1 #2"

[[feature]]
id = "hangup"
title = "A hangup ends the terminal's process"
status = "done"
milestone = "m1"
needs = ["/drivers/terminal#hangup"]
note = "PRD-0003 tier 1 #3"

[[feature]]
id = "batch"
title = "Batch jobs: the job controller starts LOGINOUT for each, with its log file"
status = "todo"
milestone = "m1"
note = "PRD-0003 step 9: JOB_CONTROL starts it (t8y:/utils/job-controller)"

[[feature]]
id = "restrictions"
title = "Account restrictions, password expiry, last login times, LGICMD"
status = "todo"
note = "ADR-0029 follow-ups; loginout.mar ponytail"

[[feature]]
id = "announce"
title = "SYS$ANNOUNCE before Username:, SYS$WELCOME after"
status = "todo"
note = "loginout.mar prints a fixed Welcome to vaxpunk"

[[feature]]
id = "intrusion"
title = "Break-in evasion: suspects, intruders, and their delay"
status = "todo"

[[feature]]
id = "audit"
title = "Login failures and breakins reported to the audit server and OPCOM"
status = "todo"
note = "Auditing is a non-goal of Milestone 1"

[[feature]]
id = "external-auth"
title = "External authentication: LDAP, Kerberos, through the ACME server"
status = "todo"

[[needs]]
on = "/rms/indexed"
why = "Reads the user's record from SYSUAF.DAT by username"

[[needs]]
on = "/exec/processes"
why = "Runs in a process of its own per terminal, which becomes the user's"

[[needs]]
on = "/exec/privileges"
why = "EXE$LOGIN sets the authorized and default privileges"

[[needs]]
on = "/exec/uic-protection"
why = "The process takes the user's UIC"

[[needs]]
on = "/exec/logical-names"
why = "Defines SYS$LOGIN, SYS$SCRATCH, SYS$DISK"

[[needs]]
on = "/drivers/terminal"
why = "Asks for a username, and a password without echo"

[[needs]]
on = "/utils/authorize"
why = "Its records are AUTHORIZE's"

[[needs]]
on = "/exec/intrusion-detection"
why = "Break-in evasion"
soft = true

[[needs]]
on = "/utils/audit-server"
why = "Login and breakin audit events"
soft = true

[[needs]]
on = "/utils/opcom"
why = "Security alarms on the operator's terminal"
soft = true

[[needs]]
on = "/net/ldap"
why = "External authentication"
soft = true

[[needs]]
on = "/exec/rights-database"
why = "Builds the process's rights list from the identifiers its user holds"
soft = true
+++

`LOGINOUT.EXE` is the image every interactive, batch and network job
starts in. On a terminal it shows `SYS$ANNOUNCE`, asks for `Username:`
and `Password:`, checks them against `SYSUAF.DAT`, applies the
account's restrictions and the break-in rules, sets the process's name,
UIC, privileges, quotas and default directory, maps the command
interpreter into P1, shows `SYS$WELCOME` and the last login times, and
exits into DCL, which runs `SYLOGIN.COM` and `LOGIN.COM`. For a batch job
it opens the log file. At `LOGOUT` it prints the accounting line and
deletes the process. The Automatic Login Facility, kept with SYSMAN,
logs certain terminals in without a password.

vaxpunk's is `vms/sysexe/loginout.mar`
([ADR-0029](../../adr/0029-loginout-and-sysuaf.md), PRD-0003 step
4). It runs in a process of its own on the console, which the swapper
starts once `STARTUP` has run `SYSTARTUP_VMS.COM`, and on each TELNET
terminal, which `TELNETD` starts. It reads the user's record by key,
checks the hash with `$HASH_PASSWORD`, counts wrong passwords, defines
`SYS$LOGIN`, `SYS$SCRATCH` and `SYS$DISK`, and calls `EXE$LOGIN` in
kernel mode, which makes the process the user's. `--autologin` writes a
text `SYSALF.DAT` for the console. Missing for Milestone 1: quotas,
`LNM$JOB`, login at a key, and batch jobs.
