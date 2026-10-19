+++
title = "ACL editor"
summary = "EDIT/ACL and SET SECURITY/ACL: edit an object's access control list"
aka = ["EDIT/ACL", "ACLEDT", "ACL editor", "SET SECURITY/ACL", "SET ACL", "SHOW ACL"]
kind = "core"
intent = "aspiration"
research = "deferred"
status = "todo"
refs = ["docs/prd/0003-multi-user-vms.md", "https://docs.vmssoftware.com/vsi-openvms-system-management-utilities-reference-manual-volume-i-a-l/"]

[[needs]]
on = "/exec/acls"
why = "The ACLs it edits, kept with files, devices, queues and logical name tables"

[[needs]]
on = "/exec/rights-database"
why = "ACEs name identifiers from RIGHTSLIST.DAT"

[[needs]]
on = "/rtl/tpushr"
why = "EDIT/ACL is a TPU section with keypad editing of ACEs"
soft = true

[[needs]]
on = "/rtl/cli"
why = "SET SECURITY/ACL's qualifiers"
+++

An access control list is a list of entries attached to a file,
directory, device, queue, logical name table or other object, each
saying which identifiers get which access, ahead of UIC protection.
`EDIT/ACL file` opens a screen editor, built on TPU, where each line is
an ACE such as `(IDENTIFIER=PAYROLL,ACCESS=READ+WRITE)`, with keypad
help to build them; `SET SECURITY/ACL=(...)` (formerly `SET ACL`) adds,
deletes or replaces entries from the command line, and `SHOW SECURITY`
lists them. Default ACEs on a directory propagate to files created in
it.

vaxpunk has ACLs on files and `SET SECURITY/ACL`
([ADR-0040](../../adr/0040-acls-in-file-headers-and-rights-identifiers.md)),
but no `EDIT/ACL`.
