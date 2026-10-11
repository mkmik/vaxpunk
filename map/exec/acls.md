+++
title = "Access control lists"
summary = "Entries naming identifiers on an object, checked before its UIC protection"
aka = ["ACL", "ACE", "EDIT/ACL", "ACL editor", "SET ACL", "SHOW ACL", "SET SECURITY/ACL", "DIRECTORY/ACL", "$CHANGE_ACL", "$FORMAT_ACL", "$PARSE_ACL", "identifier ACE", "default protection ACE", "alarm ACE", "subsystem ACE", "protected subsystem"]
kind = "core"
intent = "aspiration"
research = "outline"
refs = ["docs/prd/0003-multi-user-vms.md", "docs/design/0002-executive-processes.md"]

[[feature]]
id = "identifier-aces"
title = "Identifier ACEs on files, checked before the protection mask"
status = "todo"
needs = ["//exec/rights-database:process-rights"]

[[feature]]
id = "dcl"
title = "SET SECURITY/ACL, SHOW SECURITY and DIRECTORY/ACL"
status = "todo"
needs = [":identifier-aces"]

[[feature]]
id = "editor"
title = "The ACL editor, EDIT/ACL"
status = "todo"
needs = [":identifier-aces"]

[[feature]]
id = "inheritance"
title = "Default protection ACEs and DEFAULT entries: what a directory gives new files"
status = "todo"
needs = [":identifier-aces"]

[[feature]]
id = "other-objects"
title = "ACLs on devices, queues, logical name tables, event flag clusters, global sections"
status = "todo"
needs = ["//exec/uic-protection:object-classes"]

[[feature]]
id = "alarm-aces"
title = "Security alarm and audit ACEs"
status = "todo"
needs = ["//exec/security-audit"]

[[feature]]
id = "subsystems"
title = "Subsystem ACEs: an image that grants its identifier while it runs"
status = "todo"

[[feature]]
id = "services"
title = "$CHANGE_ACL, $FORMAT_ACL, $PARSE_ACL; ACLs in $CHKPRO and $CHECK_ACCESS"
status = "todo"

[[needs]]
on = "//exec/rights-database"
why = "An ACE grants access to the holders of identifiers"

[[needs]]
on = "//exec/uic-protection"
why = "The protection mask decides when no ACE matches"

[[needs]]
on = "//files/ods2"
why = "A file's ACL is stored in its header"
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

vaxpunk has none: ACLs are a non-goal of PRD-0003, which asks as an
open question whether they come in Milestone 1 after all; `SYSPRV` and
groups cover its cases. They need the rights database first.
