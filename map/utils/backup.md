+++
title = "BACKUP"
summary = "Saves files in save sets VMS's BACKUP reads, restores and lists them"
aka = ["BACKUP", "BACKUP.EXE", "save set", "BCK", "BACKUP/IMAGE", "BACKUP/LIST", "BACKUP/PHYSICAL", "BACKUP$_", "BACKUP/RECORD"]
kind = "core"
intent = "goal"
research = "done"
refs = ["vms/sysexe/backup.mar", "vms/cld/backup.cld", "boot/tests/boot.rs", "docs/design/0002-executive-processes.md", "https://docs.vmssoftware.com/vsi-openvms-system-management-utilities-reference-manual-volume-i-a-l/"]

[[feature]]
id = "save"
title = "Files into a disk save set, VMS's format with CRCs, as BACKUP reads it"
status = "done"
note = "PR #100"

[[feature]]
id = "restore"
title = "A save set's files restored with their names, versions, attributes, owner and protection"
status = "done"
note = "PR #100"

[[feature]]
id = "list"
title = "/LIST and /LOG"
status = "done"
note = "PR #100"

[[feature]]
id = "dates"
title = "Files' dates saved and restored"
status = "todo"
note = "backup.mar ponytail: RMS had no date XAB then; it has XABDAT now (PRD-0008 step 3)"

[[feature]]
id = "trees"
title = "[...], directories restored as a tree, /SELECT and /EXCLUDE"
status = "todo"
needs = ["//rms/wildcards:directories"]
note = "backup.mar ponytail"

[[feature]]
id = "file-to-file"
title = "File to file copies and /SINCE, /BEFORE, /RECORD for incremental saves"
status = "todo"
note = "backup.mar ponytail"

[[feature]]
id = "xor"
title = "XOR blocks, /GROUP_SIZE, so a bad block can be rebuilt"
status = "todo"
note = "backup.mar ponytail"

[[feature]]
id = "image"
title = "/IMAGE: a whole volume, saved or copied, its free space left out"
status = "todo"
needs = ["//files/ods2:qio-interface"]
note = "backup.mar ponytail"

[[feature]]
id = "physical"
title = "/PHYSICAL: a disk block for block"
status = "todo"
needs = ["//drivers/disk"]

[[feature]]
id = "tape"
title = "Save sets on tape, several volumes, /REWIND, /LABEL"
status = "todo"
note = "Needs a tape driver, //drivers/tape"

[[feature]]
id = "encrypt"
title = "/ENCRYPT and /DATA_FORMAT=COMPRESS"
status = "todo"

[[feature]]
id = "callable"
title = "Callable BACKUP, BACKUP$START"
status = "todo"

[[needs]]
on = "//rms/core"
why = "$PARSE and $SEARCH find the files; FAB and XABs give their attributes"

[[needs]]
on = "//rms/sequential"
why = "Reads and writes files with block I/O, $READ and $WRITE"

[[needs]]
on = "//files/ods2"
why = "Restores owner, protection and record attributes in the header"

[[needs]]
on = "//drivers/tape"
why = "Save sets on magnetic tape, its usual home on VMS"
soft = true

[[needs]]
on = "//rtl/cli"
why = "Its parameters and qualifiers, with /SAVE_SET placed after one"
+++

`BACKUP` is VMS's tool for saving and restoring. It writes save sets,
files of large blocks with a header, CRCs and optional XOR blocks for
recovery, on tape or disk, and reads them back. It copies trees file to
file, saves only what changed since a date or since the last
`/RECORD`ed backup, and with `/IMAGE` copies or saves a whole volume,
which is how system disks are cloned and restored, booted from CD.
`/PHYSICAL` copies block for block. `BACKUP/LIST` shows what a save set
holds. There is a callable interface, and Alpha added `/ENCRYPT` and
compression.

vaxpunk's is `vms/sysexe/backup.mar`, written so that OpenVMS's BACKUP
reads its save sets and it reads OpenVMS's: 32,256-byte blocks, a
summary record, then a `FILE` record and `VBN` records per file, each
block checked by CRC-16 and CRC-32. It saves the files a wildcard names,
lists a save set and restores its files into one directory. The boot
test saves, lists and restores. Missing: dates, `[...]`, file to file
copies, XOR blocks, `/IMAGE` and tape.
