+++
title = "Access control lists"
summary = "Entries naming identifiers on an object, checked before its UIC protection"
aka = ["ACL", "ACE", "EDIT/ACL", "ACL editor", "SET ACL", "SHOW ACL", "SET SECURITY/ACL", "DIRECTORY/ACL", "$CHANGE_ACL", "$FORMAT_ACL", "$PARSE_ACL", "identifier ACE", "default protection ACE", "alarm ACE", "subsystem ACE", "protected subsystem", "ATR$C_ADDACLENT", "FIB$L_ACLCTX"]
kind = "core"
intent = "goal"
research = "done"
refs = ["docs/adr/0040-acls-in-file-headers-and-rights-identifiers.md", "vms/exec/f11acl.mar", "vms/exec/xqp.mar", "vms/exec/vecacl.mar", "docs/prd/0003-multi-user-vms.md"]

[[feature]]
id = "identifier-aces"
title = "Identifier ACEs on files, checked before the protection mask"
status = "done"
note = "ADR-0040; FIL$CHKHDR and FIL$ACLCHK, with UIC wildcards and the rights list"

[[feature]]
id = "xqp"
title = "The XQP's ACL attributes: ADDACLENT, DELACLENT, FNDACLENT, DELETEACL, READACL, ACLLENGTH, READACE"
status = "done"
note = "xqp.mar, with FIB$L_ACLCTX and FIB$L_ACL_STATUS; no MODACLENT, FNDACETYP or the ACL locks"

[[feature]]
id = "dcl"
title = "SET SECURITY/ACL, SHOW SECURITY and DIRECTORY/ACL"
status = "done"
note = "ADR-0040"

[[feature]]
id = "editor"
title = "The ACL editor, EDIT/ACL"
status = "todo"
needs = ["#identifier-aces"]

[[feature]]
id = "inheritance"
title = "Default protection ACEs and DEFAULT entries: what a directory gives new files"
status = "done"
note = "FIL$ACLINHERIT at $CREATE; a new version gets its predecessor's ACL but NOPROPAGATE ACEs"

[[feature]]
id = "directory-inheritance"
title = "A new directory's own DEFAULT ACEs from its parent's"
status = "todo"
note = "f11acl.mar: a new file only"

[[feature]]
id = "long-acls"
title = "ACLs longer than the header's room, in extension headers"
status = "todo"
needs = ["/files/ods2#extension-headers"]
note = "f11acl.mar ponytail: SS$_ACLFULL"

[[feature]]
id = "other-objects"
title = "ACLs on devices, queues, logical name tables, event flag clusters, global sections"
status = "todo"
needs = ["/exec/uic-protection#object-classes"]

[[feature]]
id = "alarm-aces"
title = "Security alarm and audit ACEs"
status = "todo"

[[feature]]
id = "subsystems"
title = "Subsystem ACEs: an image that grants its identifier while it runs"
status = "todo"

[[feature]]
id = "services"
title = "$PARSE_ACL and $FORMAT_ACL"
status = "done"
note = "vecacl.mar, in the caller's mode; the file access names only"

[[feature]]
id = "change-acl"
title = "$CHANGE_ACL; ACLs in $CHKPRO and $CHECK_ACCESS"
status = "todo"
needs = ["/exec/uic-protection#check-services"]

[[needs]]
on = "/exec/rights-database"
why = "An ACE grants access to the holders of identifiers"

[[needs]]
on = "/exec/uic-protection"
why = "The protection mask decides when no ACE matches"

[[needs]]
on = "/files/ods2"
why = "A file's ACL is stored in its header"

[[needs]]
on = "/exec/security-audit"
why = "Alarm and audit ACEs"
soft = true
+++

An access control list is a list of access control entries on an
object. The common one, an identifier ACE, says that holders of some
identifiers, `(IDENTIFIER=PAYROLL,ACCESS=READ+WRITE)`, get some
access. When a process asks for access, VMS looks for the first ACE
whose identifiers it holds; that ACE decides, and if none matches the
UIC protection mask does. Other ACEs set the protection a directory's
new files get (`DEFAULT_PROTECTION`, and identifier ACEs with
`OPTIONS=DEFAULT`), raise security alarms or audit records on access,
or grant an identifier to whoever runs an image (subsystem ACEs, which
build protected subsystems). Files keep their ACL in the header; other
objects in memory, with templates in `VMS$OBJECTS.DAT`. They are edited
with `EDIT/ACL`, set with `SET SECURITY/ACL`, and shown with `SHOW
SECURITY` and `DIRECTORY/ACL`.

vaxpunk has them on files, as ODS-2 keeps them
([ADR-0040](../../adr/0040-acls-in-file-headers-and-rights-identifiers.md)):
the XQP's ACL attributes change them, `FIL$CHKHDR` looks at the first
identifier ACE that matches before the protection mask, and new files
get their directory's default ACEs and protection, or their previous
version's ACL. `SET SECURITY`, `SHOW SECURITY` and `DIRECTORY/ACL` and
`/SECURITY` work as OpenVMS's, through `$PARSE_ACL`, `$FORMAT_ACL`,
`$GET_SECURITY` and `$SET_SECURITY`. Missing: the ACL editor, alarm and
subsystem ACEs, ACLs on other objects, and ACLs longer than a header has
room for.
