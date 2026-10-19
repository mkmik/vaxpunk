+++
title = "TYPE"
summary = "Writes a text file on the terminal, or its first or last lines"
aka = ["TYPE", "TYPE.EXE", "TYPE/PAGE", "TYPE/TAIL", "TYPE/HEAD"]
kind = "core"
intent = "goal"
research = "done"
milestone = "m0"
refs = ["vms/sysexe/type.mar", "vms/cld/type.cld", "docs/adr/0007-system-disk-files-11-and-rms.md", "docs/prd/0003-multi-user-vms.md", "https://docs.vmssoftware.com/vsi-openvms-dcl-dictionary-n-z/"]

[[feature]]
id = "basic"
title = "A file's records on SYS$OUTPUT, a line each"
status = "done"
milestone = "m0"
note = "ADR-0007 decision 6"

[[feature]]
id = "head-tail"
title = "/HEAD=n and /TAIL=n, 10 lines by default"
status = "done"
note = "PR #117, as on OpenVMS"

[[feature]]
id = "wildcards"
title = "Wildcards and lists of files, each under its name"
status = "todo"
needs = ["/rms/wildcards#directories"]
note = "PRD-0003 tier 3 #24; type.mar ponytail"

[[feature]]
id = "long-records"
title = "Records longer than 255 characters"
status = "todo"
note = "type.mar ponytail: cut at 255"

[[feature]]
id = "page"
title = "/PAGE, /PAGE=SCROLL and CLEAR_SCREEN: a screen at a time, RETURN for more"
status = "done"
needs = ["/drivers/terminal"]
note = "As OpenVMS Alpha V8.4's, checked in AXPbox"

[[feature]]
id = "page-save"
title = "/PAGE=SAVE: the screens kept, keys to scroll back, pan and search"
status = "todo"
note = "type.mar ponytail: pages as /PAGE. VMS's is an SMG$ screen (t8y:/rtl/smg)"

[[feature]]
id = "search-output"
title = "/OUTPUT, /SEARCH and /HIGHLIGHT, /CONTINUOUS"
status = "todo"

[[feature]]
id = "remote"
title = "TYPE node::file"
status = "todo"
needs = ["/net/remote-files"]

[[needs]]
on = "/rms/sequential"
why = "Reads the file's records with $GET"

[[needs]]
on = "/rtl/cli"
why = "Its parameter and /HEAD and /TAIL"

[[needs]]
on = "/rtl/librtl"
why = "Writes each line with LIB$PUT_OUTPUT"
+++

`TYPE` writes the records of one or more files on `SYS$OUTPUT`, each
file's name first when there are several. `/PAGE` shows a screen at a
time, with keys to go back and search; `/OUTPUT` sends it to a file;
`/TAIL` shows the last lines; `/CONTINUOUS` follows a file as it grows;
`/SEARCH` and `/HIGHLIGHT` mark a string.

vaxpunk's is `vms/sysexe/type.mar`: it opens one file with RMS, the
default directory unless the spec says where, and writes each record a
line at a time, or `/HEAD`'s first and `/TAIL`'s last n, rewinding and
skipping for `/TAIL`. `/PAGE` clears the screen, keeps the last row out
of the scrolling region for its prompt, and reads RETURN or CTRL/Z there,
with escape sequences of its own, as VMS's output looks; `/PAGE=SAVE`, whose
keys go back and search, pages as `/PAGE` does. It takes one file, no wildcards
(PRD-0003 tier 3 #24), and cuts records at 255 characters.
