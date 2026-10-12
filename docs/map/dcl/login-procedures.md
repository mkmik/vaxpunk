+++
title = "Login procedures"
summary = "SYLOGIN.COM, LOGIN.COM and LGICMD; captive accounts"
aka = ["SYLOGIN.COM", "SYS$SYLOGIN", "LOGIN.COM", "LGICMD", "CAPTIVE", "RESTRICTED", "DISCTLY", "F$MODE"]
kind = "core"
intent = "goal"
research = "done"
milestone = "m1"
refs = ["docs/adr/0029-loginout-and-sysuaf.md", "vms/sysexe/dcl.mar", "vms/sysmgr/sylogin.com", "vms/sysmgr/login.com", "docs/prd/0003-multi-user-vms.md"]

[[feature]]
id = "sylogin-login"
title = "On a terminal, SYS$MANAGER:SYLOGIN.COM then SYS$LOGIN:LOGIN.COM, each if there"
status = "done"
milestone = "m1"
note = "ADR-0029 decision 5"

[[feature]]
id = "mode-test"
title = "LOGIN.COM skips its terminal setup unless F$MODE() is INTERACTIVE"
status = "done"
milestone = "m1"
note = "PRD-0003 step 8's visible result"

[[feature]]
id = "sys-sylogin"
title = "The SYS$SYLOGIN logical name says where the system's procedure is"
status = "todo"
note = "dcl.mar names SYS$MANAGER:SYLOGIN"

[[feature]]
id = "batch"
title = "Batch jobs run SYLOGIN.COM and LOGIN.COM before the job"
status = "todo"
milestone = "m1"
needs = ["/utils/job-controller"]
note = "PRD-0003 step 9; dcl.mar runs a file SYS$INPUT and logs out"

[[feature]]
id = "lgicmd"
title = "The UAF's LGICMD in place of LOGIN.COM, and /COMMAND at Username:"
status = "todo"
needs = ["/utils/loginout"]
note = "ADR-0029 follow-up"

[[feature]]
id = "captive"
title = "CAPTIVE and RESTRICTED accounts, DISCTLY: no way out of the login procedure"
status = "todo"
needs = ["/utils/loginout"]
note = "ADR-0029 follow-up: account restrictions"

[[needs]]
on = "/dcl/procedures"
why = "They are command procedures"

[[needs]]
on = "/utils/loginout"
why = "LOGINOUT sets SYS$LOGIN and gives the process DCL"

[[needs]]
on = "/dcl/lexicals"
why = "Procedures test F$MODE() and F$GETDVI"
soft = true
+++

When a user logs in, LOGINOUT and DCL run two procedures before the
first prompt: the system's, `SYS$SYLOGIN`, which sites point at
`SYS$MANAGER:SYLOGIN.COM`, then the user's, `SYS$LOGIN:LOGIN.COM`, or
the one the UAF's `LGICMD` names. Batch jobs run them too, so they test
`F$MODE()` to skip terminal setup. A captive account's login procedure
can't be left: CTRL/Y is off, the user can't name another procedure at
login, and the procedure decides what the user may do; `RESTRICTED` is
the same with fewer limits, and `DISCTLY` turns CTRL/Y off from the
start.

vaxpunk's DCL runs `@SYS$MANAGER:SYLOGIN` and `@SYS$LOGIN:LOGIN`, each
if it is there, before it prompts on a terminal, and runs a file that is
its `SYS$INPUT` as a procedure, then logs out, as `STARTUP` needs
([ADR-0029](../../adr/0029-loginout-and-sysuaf.md)). `SYLOGIN.COM`
defines `HOME`, `NSLOOKUP` and `SHUTDOWN` and sizes the console;
`SYSTEM`'s `LOGIN.COM` tests `F$MODE()`. `LGICMD` and the account
flags are in the UAF but not acted on.
