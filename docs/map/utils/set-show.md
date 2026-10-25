+++
title = "SET and SHOW"
summary = "Change and display the process, terminal, files, devices and system"
aka = ["SET", "SHOW", "SET.EXE", "SHOW.EXE", "SET PROCESS", "SHOW PROCESS", "SHOW SYSTEM", "SET TERMINAL", "SHOW TERMINAL", "SET PROTECTION", "SET FILE", "SET DIRECTORY", "SET VOLUME", "SHOW DEVICES", "SHOW LOGICAL", "SHOWLOG", "SHOW TIME", "SHOW USERS", "SHOW MEMORY", "SHOW ERROR", "SHOW QUOTA", "SHOW CPU", "SET SECURITY"]
kind = "core"
intent = "goal"
research = "done"
milestone = "m0"
refs = ["vms/sysexe/set.mar", "vms/sysexe/show.mar", "vms/sysexe/dcl/setshow.mar", "vms/cld/dcl.cld", "docs/prd/0003-multi-user-vms.md", "docs/design/0002-executive-processes.md", "https://docs.vmssoftware.com/vsi-openvms-dcl-dictionary-n-z/"]

[[feature]]
id = "show-process-system"
title = "SHOW PROCESS and SHOW SYSTEM, from $GETJPI and $GETSYI"
status = "done"
milestone = "m0"
note = "The date in both since PRD-0003 tier 1 #7"

[[feature]]
id = "show-devices"
title = "SHOW DEVICES [name] [/MOUNTED] [/FULL]: disks, volumes, free blocks, terminals"
status = "done"
milestone = "m0"
note = "DCL's own, in dcl/setshow.mar, where VMS's DCL runs SHOW.EXE"

[[feature]]
id = "privileges"
title = "SET PROCESS/PRIVILEGES and SHOW PROCESS/PRIVILEGES"
status = "done"
milestone = "m1"
note = "PRD-0003 step 1"

[[feature]]
id = "protection"
title = "SET PROTECTION=(...) file and SET FILE/OWNER_UIC"
status = "done"
milestone = "m1"
note = "PRD-0003 step 2"

[[feature]]
id = "terminal"
title = "SET TERMINAL and SHOW TERMINAL: width, page, echo, escape, pasthru, editing"
status = "done"
milestone = "m1"
note = "PR #98; per terminal UCB since PRD-0003 step 3"

[[feature]]
id = "show-time"
title = "SHOW TIME, on $BINTIM and $ASCTIM"
status = "todo"
milestone = "m1"
needs = ["/exec/time-conversion#bintim"]
note = "PRD-0003 tier 1 #7, step 5"

[[feature]]
id = "quotas-accounting"
title = "SHOW PROCESS/QUOTAS and /ACCOUNTING; CPU time and I/O counts in SHOW SYSTEM"
status = "todo"
milestone = "m1"
needs = ["/exec/quotas", "/exec/information-services#jpi-quotas"]
note = "PRD-0003 tier 1 #4, step 5; show.mar ponytail"

[[feature]]
id = "every-device"
title = "SHOW DEVICES lists every device from its UCB, BGA0: too, and /FULL"
status = "todo"
milestone = "m1"
needs = ["/exec/information-services#devices-from-ucbs"]
note = "PRD-0003 tier 1 #9 and tier 3 #24"

[[feature]]
id = "default-protection"
title = "SET PROTECTION/DEFAULT and SHOW PROTECTION: the process's default"
status = "todo"
milestone = "m1"
needs = ["/files/ods2#process-default-protection"]
note = "PRD-0003 tier 1 #8"

[[feature]]
id = "xqp-protection"
title = "SET PROTECTION without read access to the file, through the XQP"
status = "done"
needs = ["/files/ods2#acp-attributes"]
note = "PRD-0003 tier 3 #23; set.mar's IO$_ACCESS and IO$_MODIFY (ADR-0039)"

[[feature]]
id = "wildcards"
title = "Wildcards, lists and /LOG on SET PROTECTION and SET FILE"
status = "todo"
needs = ["/rms/wildcards"]
note = "PRD-0003 tier 3 #24; set.mar ponytail"

[[feature]]
id = "set-file"
title = "SET FILE/ATTRIBUTE, /VERSION_LIMIT, /EXPIRATION_DATE; SET DIRECTORY; SET VOLUME"
status = "done"
needs = ["/files/ods2#acp-attributes", "/files/ods2#version-limits", "/files/volumes#set-volume", "/exec/time-conversion#bintim"]
note = "set.mar, through IO$_ACCESS and IO$_MODIFY; docs/research/files-11-acp-qio.md. ponytail: no /LOG, /CONFIRM, wildcards or dates in /ATTRIBUTE"

[[feature]]
id = "set-process"
title = "SET PROCESS/NAME, /PRIORITY, /SUSPEND, /RESUME, /IDENTIFICATION"
status = "todo"

[[feature]]
id = "terminal-more"
title = "SET TERMINAL/INQUIRE, /DEVICE_TYPE, /SPEED, /PERMANENT, /COLOR; SHOW TERMINAL/PERMANENT; TT3$ characteristics"
status = "done"
note = "The VT52, VT100, VT102, VT200 to VT500 and UNKNOWN types, as OpenVMS Alpha V8.4's set them; set.mar ponytail: no LA or other types, /BACKSPACE, /PARITY or fill"

[[feature]]
id = "system-views"
title = "SHOW USERS, SHOW MEMORY, SHOW ERROR, SHOW QUOTA, SHOW CPU"
status = "done"
needs = ["/exec/information-services"]
note = "From $GETJPI's JPI$_MODE and JPI$_TERMINAL, which EXE$LOGIN sets, $GETSYI's memory, pool and CPU items, and DVI$_ERRCNT; SHOW QUOTA says SS$_QFNOTACT until t8y:/files/disk-quotas"

[[feature]]
id = "set-time"
title = "SET TIME, with OPER and LOG_IO"
status = "todo"
needs = ["/exec/timers"]

[[feature]]
id = "rms-default"
title = "SET RMS_DEFAULT and SHOW RMS_DEFAULT"
status = "todo"

[[feature]]
id = "security"
title = "SET SECURITY and SHOW SECURITY: owners, protection and ACLs of any object"
status = "done"
note = "ADR-0040; the class FILE only, as the services"

[[feature]]
id = "node-name"
title = "SHOW SYSTEM's header: on node NAME, as VMS's"
status = "todo"
needs = ["/exec/sysgen-parameters#scsnode"]
note = "show.mar ponytail: no SCSNODE, so no node name"

[[feature]]
id = "show-logical"
title = "SHOW LOGICAL in SHOW.EXE, installed with CMKRNL, NETMBX and WORLD: names sorted, no CMKRNL of the user's"
status = "done"
needs = ["/exec/installed-images#fixed-list", "/dcl/interpreter#ctrl-y-privileges"]
note = "As VMS's SHOWLOG reads the tables in kernel mode and sorts them (V4.3 CLIUTL SHOWLOG.B32, SYSMGR VMSIMAGES.DAT). show.mar, on process.mar's KNOWN list; SHOW PROCESS/PRIVILEGES shows JPI$_PROCPRIV, without SHOW.EXE's own. PRD-0003 tier 3 #25"

[[feature]]
id = "show-logical-translations"
title = "SHOW LOGICAL name's iterative translations, 1 to LNM$C_MAXDEPTH deep"
status = "todo"
needs = ["#show-logical"]

[[feature]]
id = "show-logical-modes"
title = "SHOW LOGICAL's access modes and attributes, [super], [exec], [concealed,terminal]; /ACCESS_MODE"
status = "todo"
needs = ["#show-logical", "/exec/logical-names#access-modes", "/exec/logical-names#attributes"]

[[feature]]
id = "show-logical-tables"
title = "SHOW LOGICAL of the tables LNM$DCL_LOGICAL lists; /JOB, /GROUP, /TABLE=*, /STRUCTURE, /FULL"
status = "todo"
needs = ["#show-logical", "/exec/logical-names#directories", "/exec/logical-names#job-group"]

[[feature]]
id = "set-privileged"
title = "SET.EXE installed with CMKRNL, SYSPRV and TMPMBX: SET VOLUME by a volume's owner without CMKRNL"
status = "todo"
needs = ["/exec/installed-images#fixed-list", "/dcl/interpreter#ctrl-y-privileges"]
note = "As VMS V4.3's VMSIMAGES.DAT installs it; vaxpunk's SET.EXE has no privileges of its own (f11wrt.mar FIL$SETVOLUME)"

[[needs]]
on = "/exec/information-services"
why = "SHOW reads $GETJPI, $GETSYI and $GETDVI"

[[needs]]
on = "/exec/privileges"
why = "SET PROCESS/PRIVILEGES calls $SETPRV"

[[needs]]
on = "/rms/core"
why = "SET PROTECTION and SET FILE change a protection XAB at $CLOSE"

[[needs]]
on = "/drivers/terminal"
why = "SET and SHOW TERMINAL: IO$_SETMODE and IO$_SENSEMODE"

[[needs]]
on = "/exec/time-conversion"
why = "$ASCTIM for the dates SHOW writes"

[[needs]]
on = "/rtl/cli"
why = "Their keywords, parameters and qualifiers"

[[needs]]
on = "/rtl/librtl"
why = "Writes their lines with LIB$PUT_OUTPUT"

[[needs]]
on = "/exec/acls"
why = "SET and SHOW SECURITY with ACLs"
soft = true
+++

`SET` and `SHOW` are two DCL verbs with dozens of options each, most of
them run as `SET.EXE` and `SHOW.EXE`. `SET` changes the process (name,
priority, privileges), the terminal, files and directories (protection,
owner, attributes, version limits), volumes, the time, RMS defaults and
security on any object. `SHOW` displays the process, the system's
processes, users, memory, devices, errors, quotas, the time, logical
names and symbols. Others belong to their own facilities: `SET HOST`,
`SET COMMAND`, `SET PASSWORD`, `SHOW QUEUE`, `SHOW CLUSTER`.

vaxpunk's `SET.EXE` is `vms/sysexe/set.mar`: `SET PROCESS/PRIVILEGES`,
`SET PROTECTION`, `SET FILE` (`/OWNER_UIC`, `/ATTRIBUTE`,
`/EXPIRATION_DATE`, `/VERSION_LIMIT`) and `SET DIRECTORY/VERSION_LIMIT`,
which ask the file system with `IO$_ACCESS` and `IO$_MODIFY`, `SET
SECURITY`, a file's owner, protection and ACL through `$SET_SECURITY`
([ADR-0040](../../adr/0040-acls-in-file-headers-and-rights-identifiers.md)),
`SET VOLUME`, and `SET TERMINAL`, which asks the terminal its type with
`/INQUIRE` or takes it from `/DEVICE_TYPE`, and sets its speeds and,
with `/PERMANENT`, its permanent characteristics.
`SHOW.EXE` is `show.mar`: `SHOW PROCESS`, with `/RIGHTS`, `SHOW SYSTEM`,
`SHOW TERMINAL`, `SHOW USERS`, `SHOW MEMORY`, `SHOW ERROR`, `SHOW QUOTA`,
`SHOW CPU`, `SHOW SECURITY` and `SHOW LOGICAL`, which, as VMS's, is
installed with `CMKRNL` to read the logical name tables in kernel mode,
so any user can list them, sorted. `SET DEFAULT`, `SHOW DEFAULT`,
`SHOW DEVICES` and `SHOW SYMBOL` are DCL's own code
(`dcl/setshow.mar`). `SET.EXE` is installed
with privileges on VMS too, and has none of its own here. PRD-0003 still wants `SHOW TIME`, quotas and
accounting figures, and every device listed from its UCB.
