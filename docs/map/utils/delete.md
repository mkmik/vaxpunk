+++
title = "DELETE and PURGE"
summary = "Deletes files, or all but the newest versions of them"
aka = ["DELETE", "PURGE", "DELETE.EXE", "DELETE/LOG", "PURGE/KEEP", "DELETE/ERASE"]
kind = "core"
intent = "goal"
research = "done"
milestone = "m0"
refs = ["vms/sysexe/delete.mar", "vms/cld/delete.cld", "docs/adr/0009-ramdisk-writable-files-11.md", "docs/prd/0003-multi-user-vms.md", "https://docs.vmssoftware.com/vsi-openvms-dcl-dictionary-a-m/", "https://docs.vmssoftware.com/vsi-openvms-dcl-dictionary-n-z/"]

[[feature]]
id = "basic"
title = "DELETE file;version, or ;* for every version, with $ERASE"
status = "done"
milestone = "m0"
note = "ADR-0009 decision 6"

[[feature]]
id = "wildcard-names"
title = "Wildcards in a name, type and version, through $SEARCH"
status = "done"
milestone = "m0"

[[feature]]
id = "log"
title = "/LOG: %DELETE-I-FILDEL for each file"
status = "done"
milestone = "m0"

[[feature]]
id = "protection"
title = "Delete access to the file and write access to its directory"
status = "done"
milestone = "m1"
note = "PRD-0003 step 2, checked by RMS's $ERASE"

[[feature]]
id = "open-files"
title = "An open file: its entry goes, and the last to close it deletes it"
status = "done"
milestone = "m1"
needs = ["/rms/sharing#erase-open"]
refs = ["docs/adr/0036-file-access-arbitration-locks.md"]
note = "PRD-0003 step 7"

[[feature]]
id = "lists"
title = "Lists of files, block counts in /LOG"
status = "todo"
note = "PRD-0003 tier 3 #24; delete.mar ponytail"

[[feature]]
id = "purge"
title = "PURGE [spec] /KEEP=n: all but the highest versions"
status = "done"
needs = ["/rms/wildcards"]

[[feature]]
id = "confirm"
title = "/CONFIRM, /SINCE, /BEFORE, /BY_OWNER, /EXCLUDE"
status = "todo"

[[feature]]
id = "erase"
title = "/ERASE: overwrite the blocks before they are freed"
status = "todo"
needs = ["/files/ods2#highwater"]

[[feature]]
id = "mailbox"
title = "DELETE/MAILBOX name: marks a permanent mailbox to go with its last channel, $DELMBX"
status = "todo"
needs = ["/exec/mailboxes"]
note = "Until then a CREATE/MAILBOX/PERMANENT mailbox stays until the system stops"

[[needs]]
on = "/rms/core"
why = "$PARSE, and $ERASE by the NAM block"

[[needs]]
on = "/rms/wildcards"
why = "$SEARCH finds each version a wildcard names"

[[needs]]
on = "/files/ods2"
why = "Removing the directory entry and freeing the header and blocks"

[[needs]]
on = "/rtl/cli"
why = "Its parameter and /LOG"
+++

`DELETE` removes files. It insists on a version, `;3` or `;*`, so a
mistyped command doesn't delete more than meant. `PURGE` deletes all but
the newest version of each file, or the newest n with `/KEEP=n`, which
is how a VMS directory is kept tidy, since every edit makes a new
version. Both take wildcards and lists, `/CONFIRM`, `/LOG`, the date and
owner selections, and `/ERASE`, which overwrites the blocks first.
`DELETE` also deletes other things: `/SYMBOL`, `/KEY`, `/ENTRY` and
`/QUEUE` are DCL's and the job controller's.

vaxpunk's `DELETE` is `vms/sysexe/delete.mar`. It requires a version,
`%DELETE-E-DELVER` otherwise, finds the files with `$PARSE` and
`$SEARCH`, and `$ERASE`s each by its NAM block; RMS checks delete access
to the file and write access to its directory. `DELETE/SYMBOL` is DCL's
own. `PURGE` runs the same image, as on VMS: it `$SEARCH`es every
version of the names, which come highest first, and `$ERASE`s all but
the first `/KEEP` of each. A file someone has open loses its directory
entry at once and is deleted when the last one closes it, as on VMS
([ADR-0036](../../adr/0036-file-access-arbitration-locks.md)).
