+++
title = "SYSMAN"
summary = "System management from one place: startup, ALF, parameters, quotas, nodes"
aka = ["SYSMAN", "SYSMAN.EXE", "SYSMAN>", "SMISERVER", "ALF", "SYSALF.DAT", "STARTUP database", "VMS$LAYERED.DAT", "DISKQUOTA", "IO AUTOCONFIGURE", "CLASS_SCHEDULE"]
kind = "core"
intent = "aspiration"
research = "outline"
refs = ["vms/sysexe/loginout.mar", "docs/prd/0003-multi-user-vms.md", "https://docs.vmssoftware.com/vsi-openvms-system-management-utilities-reference-manual-volume-ii-m-z/", "https://h41379.www4.hpe.com/doc/84final/6048/6048pro_074.html"]

[[feature]]
id = "alf"
title = "ALF ADD, REMOVE, SHOW: the Automatic Login Facility's SYSALF.DAT"
status = "todo"
needs = ["//rms/indexed"]
note = "loginout.mar ponytail: SYSALF.DAT is a text file boot --autologin writes, until there is a SYSMAN"

[[feature]]
id = "startup-database"
title = "STARTUP ADD, SHOW, ENABLE, DISABLE: the startup database of components"
status = "todo"

[[feature]]
id = "parameters"
title = "PARAMETERS SHOW, SET, USE, WRITE"
status = "todo"
needs = ["//exec/sysgen-parameters"]

[[feature]]
id = "diskquota"
title = "DISKQUOTA CREATE, ADD, MODIFY, SHOW, REBUILD"
status = "todo"
needs = ["//files/disk-quotas"]

[[feature]]
id = "io"
title = "IO AUTOCONFIGURE, CONNECT, LOAD, SHOW DEVICE: drivers and devices, Alpha's way"
status = "todo"
needs = ["//exec/qio"]

[[feature]]
id = "license"
title = "LICENSE LOAD and UNLOAD"
status = "todo"

[[feature]]
id = "class-schedule"
title = "CLASS_SCHEDULE: CPU time shares for classes of users"
status = "todo"
needs = ["//exec/scheduler"]

[[feature]]
id = "environment"
title = "SET ENVIRONMENT, DO and SHUTDOWN NODE: commands on several nodes at once"
status = "todo"
note = "Clusters are a non-goal of Milestone 1"

[[needs]]
on = "//rtl/cli"
why = "Its own commands at the SYSMAN> prompt"

[[needs]]
on = "//exec/privileges"
why = "Most commands take CMKRNL, SYSPRV or OPER"

[[needs]]
on = "//utils/startup-shutdown"
why = "STARTUP ADD and SHOW keep the startup database STARTUP.COM reads"
soft = true

[[needs]]
on = "//utils/license-manager"
why = "LICENSE LOAD and UNLOAD"
soft = true

[[needs]]
on = "//exec/cluster"
why = "SET ENVIRONMENT/CLUSTER and DO on every node"
soft = true

[[needs]]
fuzzy = "SMISERVER, the server process that runs SYSMAN's commands on each node"
why = "SET ENVIRONMENT and DO reach other nodes through it"
+++

`SYSMAN` was added in VMS V5 to manage a system, or every node of a
cluster, from one prompt. `SET ENVIRONMENT` picks the nodes, and `DO`
runs a DCL command on each through a server process, `SMISERVER`. Its
other commands gathered work that had been spread around: the startup
database that orders layered products' startup procedures, the
Automatic Login Facility, disk quotas, licenses, system parameters
(with SYSGEN), and on Alpha device configuration (`IO AUTOCONFIGURE`,
`IO CONNECT`). Later versions added class scheduling, reserved memory,
dump priorities and cluster security settings.

vaxpunk has no `SYSMAN`. The Automatic Login Facility works without it:
`LOGINOUT` reads a text `SYSALF.DAT` that `just boot --autologin`
writes, where VMS's is an indexed file SYSMAN keeps.
