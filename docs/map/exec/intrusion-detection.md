+++
title = "Intrusion detection"
summary = "Break-in detection and evasion: login failures tracked by where they come from"
aka = ["break-in", "breakin evasion", "intrusion database", "SHOW INTRUSION", "DELETE/INTRUSION_RECORD", "LGI_BRK_LIM", "LGI_BRK_TMO", "LGI_HID_TIM", "LGI_BRK_DISUSER", "LGI_RETRY_LIM", "SECURITY_SERVER", "$SCAN_INTRUSION", "$SHOW_INTRUSION", "$DELETE_INTRUSION", "suspect", "intruder"]
kind = "core"
intent = "aspiration"
research = "outline"
refs = ["docs/adr/0029-loginout-and-sysuaf.md", "vms/sysexe/loginout.mar"]

[[feature]]
id = "database"
title = "The intrusion database: failures by terminal, username and remote host"
status = "todo"

[[feature]]
id = "evasion"
title = "Evasion: an intruder's logins refused, right password or not, for LGI_HID_TIM"
status = "todo"
needs = ["#database", "/utils/loginout"]
note = "LOGINOUT asks the database before it checks the password"

[[feature]]
id = "disuser"
title = "LGI_BRK_DISUSER: an account under attack disabled"
status = "todo"
needs = ["#database"]

[[feature]]
id = "show-delete"
title = "SHOW INTRUSION and DELETE/INTRUSION_RECORD; $SHOW_INTRUSION, $SCAN_INTRUSION, $DELETE_INTRUSION"
status = "todo"
needs = ["#database"]

[[feature]]
id = "network"
title = "Network logins (TELNET, FTP, SSH) counted by remote host and username"
status = "todo"
needs = ["#database"]

[[feature]]
id = "clusterwide"
title = "One database for the whole cluster"
status = "todo"
needs = ["#database", "/exec/cluster"]
note = "SECURITY_POLICY says local or cluster-wide"

[[needs]]
on = "/exec/sysgen-parameters"
why = "LGI_BRK_LIM, LGI_BRK_TMO, LGI_HID_TIM and the rest set its limits"

[[needs]]
on = "/exec/security-audit"
why = "BREAKIN and LOGFAILURE alarms"
soft = true

[[needs]]
fuzzy = "SECURITY_SERVER, the process that keeps the intrusion database"
why = "The database is a server's, so a cluster can share it"
+++

VMS watches login failures by their source: a terminal, a username, a
remote host, or a pair of them. A source with a failure becomes a
suspect; past `LGI_BRK_LIM` failures within `LGI_BRK_TMO` it becomes an
intruder, and for `LGI_HID_TIM` its logins are refused even with the
right password, without saying so, which defeats guessing. With
`LGI_BRK_DISUSER` the account itself is disabled. `SHOW INTRUSION`
lists the records and `DELETE/INTRUSION_RECORD` clears one. The
database is kept by the `SECURITY_SERVER` process and can be shared by
a whole cluster. Break-ins also raise audit alarms
(`t8y:/exec/security-audit`).

vaxpunk has no intrusion database. What it has is `LOGINOUT`'s own
retry limit: three tries per connection, and a wrong password counted
in the UAF's `UAF$W_LOGFAILS`, which the next login reports
([ADR-0029](../../adr/0029-loginout-and-sysuaf.md), PRD-0003
step 4). A wrong username and a wrong password look the same on the
screen, as on VMS.
