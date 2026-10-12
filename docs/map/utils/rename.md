+++
title = "RENAME"
summary = "Changes a file's name, type, version or directory on the same volume"
aka = ["RENAME", "RENAME.EXE", "$RENAME", "LIB$RENAME_FILE"]
kind = "core"
intent = "aspiration"
research = "outline"
status = "todo"
refs = ["https://docs.vmssoftware.com/vsi-openvms-dcl-dictionary-n-z/"]

[[needs]]
on = "/rms/core#rename"
why = "RMS's $RENAME, which vaxpunk's RMS doesn't have yet"

[[needs]]
on = "/rms/wildcards"
why = "Wildcards in the input, and in the output standing for the input's parts"

[[needs]]
on = "/files/ods2"
why = "Enters the file in its new directory and removes the old entry, without copying it"

[[needs]]
on = "/rtl/cli"
why = "Its parameters and qualifiers"
+++

`RENAME old new` changes any part of a file's specification but the
device: its name, type, version, or the directory it is entered in. The
file's blocks don't move; RMS's `$RENAME` enters its header in the new
directory and removes the old entry. Wildcards in the output stand for
the matching parts of each input, so `RENAME *.TXT *.OLD` renames a set.
It takes `/LOG`, `/CONFIRM`, `/NEW_VERSION` and the date and owner
selections, and renames directories too, which VMS treats as files of
type `.DIR`. Programs call `LIB$RENAME_FILE` for the same.

vaxpunk has no `RENAME` and no `$RENAME` in RMS. PRD-0003 asks that a
VMS user from 1990 find nothing missing from an ordinary day, which this
is part of, but no PRD lists it. WASMRUN's file milestone needs
`$RENAME` too (PRD-0007 M6).
