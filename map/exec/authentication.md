+++
title = "Authentication services"
summary = "$HASH_PASSWORD, $GETUAI and $SETUAI, password policy, and $ACM with ACME agents"
aka = ["$HASH_PASSWORD", "UAI$C_PURDY_S", "UAI$C_SHA256", "$GETUAI", "$SETUAI", "$ACM", "SYS$ACM", "ACME_SERVER", "ACME agent", "VMS$VMS_ACMESHR", "LDAP ACME", "Kerberos ACME", "external authentication", "password history", "password dictionary", "VMS$PASSWORD_HISTORY.DATA", "VMS$PASSWORD_DICTIONARY", "VMS$PASSWORD_POLICY", "generated passwords"]
kind = "core"
intent = "goal"
research = "outline"
refs = ["docs/adr/0029-loginout-and-sysuaf.md", "vms/exec/hashpwd.mar", "vms/uafhash.rs", "vms/sysexe/loginout.mar", "vms/sysexe/authorize.mar", "vms/sysexe/setpwd.mar", "crosstools/vtools/lib/starlet.mlb"]

[[feature]]
id = "hash-password"
title = "$HASH_PASSWORD with UAI$C_SHA256: salted SHA-256, 4096 rounds, 8 bytes in UAF$Q_PWD"
status = "done"
milestone = "m1"
note = "PRD-0003 step 4 and open question, ADR-0029 decision 2; hashpwd.mar, and uafhash.rs on the host"

[[feature]]
id = "marked"
title = "UAI$C_SHA256 marked as a departure from VMS in its comments"
status = "todo"
note = "ADR-0031 consequences: such places get a vaxpunk: paragraph when next changed"

[[feature]]
id = "getuai"
title = "$GETUAI and $SETUAI: a user's UAF record through a service, not RMS"
status = "todo"
note = "LOGINOUT, AUTHORIZE and SETPWD read SYSUAF.DAT with RMS now"

[[feature]]
id = "expiry"
title = "Password lifetime, minimum length, expiry and pre-expired passwords"
status = "todo"
note = "ADR-0029 follow-up: password expiry; loginout.mar ponytail"

[[feature]]
id = "history-dictionary"
title = "Password history and dictionary checks, and a site's own policy module"
status = "todo"

[[feature]]
id = "generated"
title = "Generated passwords: SET PASSWORD/GENERATE and the UAF's GENPWD flag"
status = "todo"
needs = ["//exec/entropy"]

[[feature]]
id = "acm"
title = "$ACM and ACME_SERVER: logins checked by authentication agents, the VMS ACME first"
status = "todo"

[[feature]]
id = "external-agents"
title = "LDAP and Kerberos ACME agents, and a user's UAF record mapped from a directory"
status = "todo"
needs = [":acm"]
note = "//net/ldap and //net/kerberos have the agents"

[[needs]]
on = "//net/ldap"
why = "The LDAP ACME agent checks passwords against a directory"
soft = true

[[needs]]
on = "//net/kerberos"
why = "The Kerberos ACME agent accepts Kerberos credentials"
soft = true

[[needs]]
on = "//exec/system-services"
why = "Each is a service in the vector's Security group"

[[needs]]
on = "//exec/privileges"
why = "$GETUAI of another user and $SETUAI take SYSPRV, or write access to the UAF"

[[needs]]
on = "//rms/indexed"
why = "$GETUAI, $SETUAI and the password history are indexed files"
soft = true
+++

VMS keeps passwords as hashes in the UAF. `$HASH_PASSWORD` makes one
from the password, the username and a salt, with the algorithm the
record names: VAX/VMS V4's Purdy polynomial and its variants, which
are broken now. `$GETUAI` and `$SETUAI` read and write a user's UAF
record through a service, so programs need not open `SYSUAF.DAT`.
Password policy covers minimum length, lifetime and expiry, generated
passwords, a history file so old passwords can't come back, a
dictionary of words refused, and a site's own policy module. In the
V8 releases (a kit for V8.2, built in from V8.3), `$ACM`, the
authentication and credential management service, moves the checking
out of `LOGINOUT` into
`ACME_SERVER` and its agents: the VMS agent checks the UAF, others
check an LDAP directory or Kerberos, so one password serves VMS and
the rest of a site.

vaxpunk's `$HASH_PASSWORD` (`vms/exec/hashpwd.mar`) takes one
algorithm of its own, `UAI$C_SHA256`, number 128: SHA-256 of the salt,
the username and the password, 4096 times over, its first 8 bytes in
`UAF$Q_PWD`, which keeps VMS's record layout
([ADR-0029](../../docs/adr/0029-loginout-and-sysuaf.md)).
`vms/uafhash.rs` does the same on the host for the build's `SYSTEM`
record. `LOGINOUT`, `AUTHORIZE` and `SET PASSWORD` call it and read
the UAF with RMS; there is no `$GETUAI`, password policy or `$ACM`.
