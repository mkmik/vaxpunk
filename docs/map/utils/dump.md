+++
title = "DUMP"
summary = "Shows a file, a file header or disk blocks in hex, octal, decimal or ASCII"
aka = ["DUMP", "DUMP.EXE", "DUMP/HEADER", "DUMP/RECORDS", "DUMP/BLOCKS", "DUMP/FILE_HEADER", "DUMP/DIRECTORY", "RMSDUMP"]
kind = "core"
intent = "aspiration"
research = "outline"
status = "todo"
refs = ["vms/sysexe/rmsdump.mar", "crosstools/ods/README.md", "https://docs.vmssoftware.com/vsi-openvms-dcl-dictionary-a-m/", "https://wasd.vsm.com.au/htbin/helpgate/HELP/DUMP"]

[[needs]]
on = "/rms/core"
why = "Reads a file by blocks or by records"

[[needs]]
on = "/files/ods2#qio-interface"
why = "/HEADER and /FILE_HEADER read the file header through the XQP"

[[needs]]
on = "/drivers/disk"
why = "Dumping a mounted or foreign volume's logical blocks"
soft = true

[[needs]]
on = "/rtl/cli"
why = "Its parameter and qualifiers"
+++

`DUMP` shows the bytes of a file, a disk or a tape in hexadecimal,
octal, decimal or ASCII, a block or a record at a time (`/BLOCKS`,
`/RECORDS`). `/HEADER` adds the Files-11 file header, decoded field by
field: map, owner, protection, dates, record attributes. `/DIRECTORY`
decodes a directory file's records. On a device it dumps logical
blocks; on CD-ROM, ISO 9660 path tables. It is the tool system
managers and support use to look at a damaged file or a volume
structure.

vaxpunk has no `DUMP`. `RMSDUMP`, a test program, prints a file's
records with nonprinting bytes escaped, and on the host `ods dump` and
`vdump` decode volumes, headers and images.
