+++
title = "AUTHORIZE"
summary = "Maintains the user accounts in SYSUAF.DAT, and the rights database"
aka = ["AUTHORIZE", "UAF", "UAF>", "SYSUAF.DAT", "AUTHORIZE.EXE", "MCR AUTHORIZE", "RIGHTSLIST.DAT", "NETPROXY.DAT", "SET PASSWORD", "SETPWD"]
kind = "core"
intent = "goal"
research = "done"
milestone = "m1"
refs = ["vms/sysexe/authorize.mar", "vms/sysexe/authorize.cld", "vms/sysexe/setpwd.mar", "vms/sysuaf.fdl", "vms/uafhash.rs", "docs/adr/0029-loginout-and-sysuaf.md", "docs/prd/0003-multi-user-vms.md", "https://docs.vmssoftware.com/vsi-openvms-system-management-utilities-reference-manual-volume-i-a-l/"]

[[feature]]
id = "basic"
title = "ADD, MODIFY, REMOVE, SHOW and LIST, at UAF> or on the command line"
status = "done"
milestone = "m1"
note = "PRD-0003 step 4, ADR-0029"

[[feature]]
id = "qualifiers"
title = "/PASSWORD, /UIC, /PRIVILEGES, /DEFPRIVILEGES, /DEVICE, /DIRECTORY, /CLI, /PRIORITY, quotas"
status = "done"
milestone = "m1"
note = "PRD-0003 Login"

[[feature]]
id = "hash"
title = "Passwords hashed by $HASH_PASSWORD, salted, iterated SHA-256"
status = "done"
milestone = "m1"
note = "ADR-0029 decision 2: UAI$C_SHA256, a number VMS doesn't use"

[[feature]]
id = "set-password"
title = "SET PASSWORD changes one's own, with SYSPRV from the KNOWN list"
status = "done"
milestone = "m1"
note = "ADR-0029 decision 6; PRD-0003 tier 1 #2: until INSTALL"

[[feature]]
id = "salt-entropy"
title = "Salts from $GET_ENTROPY rather than the clock"
status = "todo"
needs = ["/exec/entropy"]
note = "authorize.mar and setpwd.mar take the salt from $GETTIM"

[[feature]]
id = "persistent-uaf"
title = "SYSUAF.DAT kept across boots without a site's DEFINE"
status = "todo"
note = "ADR-0029: the copy on the ramdisk is lost at each boot"

[[feature]]
id = "shared-updates"
title = "Two updates at once don't lose one"
status = "todo"
milestone = "m1"
needs = ["/rms/sharing"]
note = "ADR-0029 consequences; PRD-0003 step 7"

[[feature]]
id = "flags-restrictions"
title = "/FLAGS (DISUSER, CAPTIVE, ...), login times, /LGICMD, password lifetime and history"
status = "todo"
note = "authorize.mar ponytail; ADR-0029 follow-ups"

[[feature]]
id = "copy-rename"
title = "COPY and RENAME of a record; wildcards beyond *"
status = "todo"
note = "authorize.mar ponytail"

[[feature]]
id = "rights"
title = "Identifiers: ADD/IDENTIFIER, GRANT/IDENTIFIER, RIGHTSLIST.DAT"
status = "todo"
note = "PRD-0003: a rights list of the UIC alone"

[[feature]]
id = "proxies"
title = "Network proxies, ADD/PROXY and NETPROXY.DAT"
status = "todo"
note = "For VMS-to-VMS file access (t8y:/net/remote-files), whose server checks the UAF"

[[feature]]
id = "purdy"
title = "Purdy's polynomial hashes, to read a real VMS SYSUAF.DAT"
status = "dropped"
note = "ADR-0029 and PRD-0003: it is broken, and nothing reads our UAF but us"

[[needs]]
on = "/rms/indexed"
why = "SYSUAF.DAT is an indexed file keyed by username and UIC"

[[needs]]
on = "/exec/privileges"
why = "Writes $PRVDEF masks; only SYSTEM may write the file"

[[needs]]
on = "/exec/uic-protection"
why = "The file's protection, (S:RWE,O:RWE,G,W), keeps others out"

[[needs]]
on = "/rtl/cli"
why = "Its own command tables, AUTHORIZE.CLD"

[[needs]]
on = "/exec/rights-database"
why = "Identifiers it adds and grants"
soft = true

[[needs]]
on = "/host/ods"
why = "The build writes SYSTEM's and DEFAULT's records on the host"
soft = true
+++

`AUTHORIZE` is the system manager's tool for user accounts, records in
`SYS$SYSTEM:SYSUAF.DAT`, an indexed file in `$UAFDEF`'s layout. At its
`UAF>` prompt `ADD`, `MODIFY`, `COPY`, `RENAME`, `REMOVE`, `SHOW` and
`LIST` work on records: username, password hash, UIC, account, default
device and directory, command interpreter, privileges, quotas, priority,
login flags such as `DISUSER` and `CAPTIVE`, login times and password
lifetimes. It also keeps the rights database, `RIGHTSLIST.DAT`, whose
identifiers are granted to users and named in ACLs, and network
proxies, `NETPROXY.DAT`. `SET PASSWORD` lets a user change their own.

vaxpunk's is `vms/sysexe/authorize.mar`, with its own tables in
`authorize.cld` ([ADR-0029](../../adr/0029-loginout-and-sysuaf.md),
PRD-0003 step 4): `ADD` from `DEFAULT`'s record, `MODIFY`, `REMOVE`,
`SHOW` and `LIST` with the qualifiers PRD-0003 asks for. Passwords are
salted, iterated SHA-256, not Purdy's polynomial. The build writes a UAF
with `SYSTEM` and `DEFAULT`; `SYSTARTUP_VMS.COM` copies it to the
ramdisk, where it can change until the system stops. `SET PASSWORD` is
`SETPWD.EXE`, which the executive gives `SYSPRV` from a fixed list
until there is `INSTALL`. Missing: flags, restrictions, identifiers and
proxies.
