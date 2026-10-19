+++
title = "UICs and protection"
summary = "User identification codes, owners and SOGW protection masks on objects"
aka = ["UIC", "[g,m]", "SOGW", "protection mask", "protection code", "FIL$CHKPRO", "$CHKPRO", "SET PROTECTION", "SET FILE/OWNER_UIC", "MAXSYSGROUP", "$SETDFPROT", "SET PROTECTION/DEFAULT", "security class", "VMS$OBJECTS.DAT", "$GET_SECURITY", "$SET_SECURITY", "SET SECURITY"]
kind = "core"
intent = "goal"
research = "done"
milestone = "m1"
refs = ["docs/prd/0003-multi-user-vms.md", "docs/design/0002-executive-processes.md", "vms/exec/f11.mar", "vms/exec/f11wrt.mar", "vms/exec/process.mar", "vms/sysexe/set.mar", "vms/sysexe/prottest.mar", "vms/cld/dcl.cld"]

[[feature]]
id = "process-uic"
title = "A UIC per process, [1,4] for SYSTEM, inherited by $CREPRC; DETACH for another"
status = "done"
milestone = "m1"
note = "PRD-0003 step 1; PCB$L_UIC"

[[feature]]
id = "files"
title = "Files' owner and mask checked by FIL$CHKPRO; SET PROTECTION, SET FILE/OWNER_UIC"
status = "done"
milestone = "m1"
note = "PRD-0003 step 2; RMS checks $OPEN, $CREATE, $ERASE and each directory looked in; DIRECTORY/OWNER/PROTECTION"

[[feature]]
id = "volumes"
title = "Volumes' owner and mask: INITIALIZE and MOUNT /OWNER_UIC and /PROTECTION"
status = "done"
milestone = "m1"
note = "PRD-0003 step 2"

[[feature]]
id = "system-groups"
title = "UIC groups up to MAXSYSGROUP, 8, get the system category"
status = "done"
note = "f11.mar; a constant, not yet a system parameter"

[[feature]]
id = "default-protection"
title = "The process's default file protection: $SETDFPROT, SET PROTECTION/DEFAULT"
status = "todo"
milestone = "m1"
note = "PRD-0003 tier 1 #8; FIL$CREHDR uses VMS's default, FILEPROT; LOGINOUT should set it"

[[feature]]
id = "mailboxes"
title = "Mailboxes owned by their creator, with a protection mask"
status = "todo"
milestone = "m1"
note = "PRD-0003 tier 1 #1; mbdriver.mar ignores $CREMBX's promsk"

[[feature]]
id = "logical-name-tables"
title = "Owner and protection on logical name tables, LNM$GROUP's and LNM$JOB's first"
status = "todo"
note = "With t8y:/exec/logical-names' tables (PRD-0003 tier 3 #25)"

[[feature]]
id = "event-flag-clusters"
title = "$ASCEFC's protection on common event flag clusters"
status = "todo"
note = "DESIGN-0002: ignored now"

[[feature]]
id = "devices"
title = "Protection on devices that aren't volumes: SET DEVICE/PROTECTION"
status = "todo"

[[feature]]
id = "volume-create"
title = "A volume's third bit means create, not execute"
status = "todo"
note = "DESIGN-0002 Files ponytail"

[[feature]]
id = "security-services"
title = "$GET_SECURITY and $SET_SECURITY; SET and SHOW SECURITY"
status = "done"
note = "ADR-0040; the class FILE only, SS$_NOCLASS for the others"

[[feature]]
id = "check-services"
title = "$CHKPRO and $CHECK_ACCESS"
status = "todo"

[[feature]]
id = "object-classes"
title = "Security classes and VMS$OBJECTS.DAT: templates for queues, global sections, capabilities"
status = "todo"

[[feature]]
id = "named-uics"
title = "Named UICs, [JOE] for [200,1], from UIC identifiers"
status = "done"
note = "t8y:/exec/rights-database; $PARSE_ACL and $FORMAT_ACL"

[[needs]]
on = "/exec/privileges"
why = "SYSPRV, GRPPRV, READALL and BYPASS change the result"

[[needs]]
on = "/exec/processes"
why = "The UIC is in the PCB"
soft = true

[[needs]]
on = "/exec/rights-database"
why = "UIC identifiers"
soft = true
+++

Every VMS process has a user identification code, `[group,member]`,
two octal numbers; users who share a group number are a group. Every
protected object (file, volume, device, mailbox, logical name table,
queue, global section, common event flag cluster) has an owner UIC and
a protection mask: read, write, execute and delete (create, on a
volume) for four categories, system, owner, group and world, a set bit
denying. A process gets the world's access; the group's too if its
group is the owner's; the owner's, and control, if its UIC is the
owner's; the system's, and control, if its group is at most
`MAXSYSGROUP` or it holds `SYSPRV`. Control is the right to change the
protection. VMS V6 made this uniform: each object belongs to a security
class, `$GET_SECURITY` and `$SET_SECURITY` read and change any
object's owner, mask and ACL, and `SET SECURITY` took over from `SET
PROTECTION`. An ACL (`t8y:/exec/acls`), when an object has one, is looked
at first.

vaxpunk has UICs and file and volume protection from PRD-0003 steps 1
and 2. `FIL$CHKPRO` in `vms/exec/f11.mar` applies VMS's rules to a
header's owner and mask after its volume's; RMS checks on `$OPEN`,
`$CREATE` and `$ERASE` and for each directory a lookup reads, and the
image activator wants execute access. `PROTTEST` checks it from a
process of another UIC. Left for Milestone 1: mailboxes' masks and the
process's default protection. `$GET_SECURITY`, `$SET_SECURITY`, `SET
SECURITY` and `SHOW SECURITY` take the class `FILE`, and owners show by
name, `[SYSTEM]`
([ADR-0040](../../adr/0040-acls-in-file-headers-and-rights-identifiers.md));
the other object classes, `$CHKPRO` and `$CHECK_ACCESS` are not planned
yet.
