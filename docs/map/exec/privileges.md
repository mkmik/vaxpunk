+++
title = "Privileges"
summary = "Per-process privilege masks that let a process past the system's checks"
aka = ["$PRVDEF", "PRV$V_", "$SETPRV", "SET PROCESS/PRIVILEGES", "SHOW PROCESS/PRIVILEGES", "CMKRNL", "SETPRV", "SYSPRV", "BYPASS", "IFPRIV", "SS$_NOPRIV", "SS$_NOTALLPRIV", "authorized privileges", "image privileges"]
kind = "core"
intent = "goal"
research = "done"
milestone = "m1"
refs = ["docs/prd/0003-multi-user-vms.md", "docs/design/0002-executive-processes.md", "docs/adr/0029-loginout-and-sysuaf.md", "vms/exec/process.mar", "vms/exec/syssrv.mar", "vms/exec/f11.mar", "vms/sysexe/set.mar", "vms/sysexe/lib/prvnam.mar", "crosstools/vtools/lib/lib.mlb"]

[[feature]]
id = "masks"
title = "Current, permanent and authorized masks in the PCB; $SETPRV"
status = "done"
milestone = "m1"
note = "PRD-0003 step 1; PCB$Q_PRIV, PCB$Q_PROCPRIV, PCB$Q_AUTHPRIV; SS$_NOTALLPRIV without SETPRV"

[[feature]]
id = "dcl"
title = "SET PROCESS/PRIVILEGES and SHOW PROCESS/PRIVILEGES"
status = "done"
milestone = "m1"
note = "PRD-0003 step 1; set.mar, show.mar, names from prvnam.mar"

[[feature]]
id = "service-checks"
title = "CMKRNL, CMEXEC, SETPRV, GROUP, WORLD, DETACH, SYSNAM, PRMMBX, TMPMBX, LOG_IO, PHY_IO, VOLPRO, OPER"
status = "done"
milestone = "m1"
note = "PRD-0003 step 1; DESIGN-0002's Privileges table"

[[feature]]
id = "file-checks"
title = "SYSPRV, GRPPRV, READALL and BYPASS in file and volume protection"
status = "done"
milestone = "m1"
note = "PRD-0003 step 2; FIL$CHKPRO in f11.mar"

[[feature]]
id = "login"
title = "LOGINOUT gives a user the UAF's authorized and default privileges"
status = "done"
milestone = "m1"
note = "PRD-0003 step 4, ADR-0029; EXE$LOGIN"

[[feature]]
id = "known-images"
title = "Known images get privileges while they run: a fixed list, SETPWD.EXE"
status = "done"
note = "process.mar KNOWN; PRD-0003 tier 1 #2 keeps it until INSTALL"

[[feature]]
id = "syslck"
title = "SYSLCK for $ENQ's LCK$M_SYSTEM"
status = "todo"
milestone = "m1"
note = "PRD-0003 step 6; checked in t8y:/exec/lock-manager"

[[feature]]
id = "grpnam"
title = "GRPNAM for names in LNM$GROUP"
status = "todo"
milestone = "m1"
note = "PRD-0003 Privileges, UICs and protection; LNM$GROUP is tier 1 (tier 3 #25); checked in t8y:/exec/logical-names"

[[feature]]
id = "installed"
title = "Images installed /PRIVILEGED in place of the KNOWN list"
status = "todo"
note = "ADR-0029 follow-up; PRD-0003 tier 1 #2, tier 3 #20; t8y:/exec/installed-images, a PRD-0003 non-goal"

[[feature]]
id = "altpri"
title = "ALTPRI to set a priority above the UAF's base"
status = "todo"
note = "$SETPRI checks no privilege now (process.mar)"

[[feature]]
id = "the-rest"
title = "The rest as their services come: PRMCEB, PRMGBL, SYSGBL, PFNMAP, SHMEM, EXQUOTA, NETMBX, BUGCHK, AUDIT, ..."
status = "todo"
note = "Their bits are in starlet.mlb's $PRVDEF; nothing checks them yet"

[[needs]]
on = "/exec/processes"
why = "The masks live in the PCB"
soft = true

[[needs]]
on = "/exec/installed-images"
why = "Privileges from INSTALL/PRIVILEGED, not a fixed list"
soft = true
+++

A VMS process holds privileges, the bits of `$PRVDEF`, each of which
lets it do one thing the system otherwise refuses: change mode to
kernel (`CMKRNL`), touch other processes (`GROUP`, `WORLD`), make
system logical names (`SYSNAM`), do logical or physical I/O (`LOG_IO`,
`PHY_IO`), read or write any file (`READALL`, `SYSPRV`, `BYPASS`).
The UAF says which privileges a user is authorized for and which are
on at login. `$SETPRV` and `SET PROCESS/PRIVILEGES` turn authorized
ones on and off; `SETPRV` lets a process turn on any. An image
installed with privileges has them while it runs, and the process goes
back to its permanent ones when it exits. A service checks with
`IFPRIV` and fails with `SS$_NOPRIV`. The security manual sorts
privileges by the harm they can do, from normal (`TMPMBX`, `NETMBX`)
to all (`CMKRNL`, `SETPRV`, `BYPASS`), since several of them amount to
having every privilege. Later Alpha releases keep the masks in a
persona (`t8y:/exec/personas`).

vaxpunk has them since PRD-0003 step 1: the three masks in the PCB,
`$SETPRV` in `vms/exec/process.mar`, the `IFPRIV` and `IFNPRIV` macros
in `lib.mlb`, and `SET PROCESS/PRIVILEGES` and `SHOW
PROCESS/PRIVILEGES`. The services that exist check what VMS checks
(DESIGN-0002's table); step 2 added `SYSPRV`, `GRPPRV`, `READALL` and
`BYPASS` to file protection. The swapper has every privilege, and
`LOGINOUT` gives a user the UAF's
([ADR-0029](../../adr/0029-loginout-and-sysuaf.md)). One image,
`SETPWD.EXE`, gets `SYSPRV` while it runs from a fixed list in
`process.mar`, standing in for `INSTALL /PRIVILEGED`. Left for
Milestone 1: `SYSLCK` with the lock manager and `GRPNAM` with
`LNM$GROUP`.
