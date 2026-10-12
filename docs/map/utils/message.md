+++
title = "MESSAGE"
summary = "The message compiler: a facility's condition values and texts from a .MSG file"
aka = ["MESSAGE", "MESSAGE.EXE", ".MSG", ".FACILITY", ".SEVERITY", "message file", "message pointer file", "SET MESSAGE", "SYS$MESSAGE", "$PUTMSG"]
kind = "core"
intent = "aspiration"
research = "outline"
refs = ["vms/exec/getmsg.mar", "docs/design/0002-executive-processes.md", "docs/prd/0003-multi-user-vms.md", "docs/prd/0007-wasmrun.md"]

[[feature]]
id = "compiler"
title = "Compile a .MSG file (.FACILITY, .SEVERITY, .BASE, messages with FAO) into an object module"
status = "todo"

[[feature]]
id = "image-section"
title = "Message sections linked into an image, which $GETMSG searches"
status = "todo"
needs = ["#compiler", "/exec/messages#message-files"]
note = "PRD-0003 tier 3 #26: one message table in the executive (getmsg.mar)"

[[feature]]
id = "pointer-files"
title = "Message pointer files and SET MESSAGE, a process's own message files"
status = "todo"
needs = ["#image-section"]

[[feature]]
id = "sdl"
title = "/SDL: symbol definitions for MACRO-32, BLISS and C from the same file"
status = "todo"
needs = ["#compiler"]

[[needs]]
on = "/exec/messages"
why = "$GETMSG and $PUTMSG read the messages it compiles"

[[needs]]
on = "/host/vlink"
why = "Message object modules are linked into images"

[[needs]]
fuzzy = "A message compiler on the host, so vms/build.rs can compile .MSG files before a native one exists"
why = "Every utility's messages come from a .MSG file built with the system"
+++

Each VMS facility (RMS, SYSTEM, DCL, a user's program) declares its
condition values and their texts in a message source file: `.FACILITY`
names it and gives its number, `.SEVERITY` sets success, informational,
warning, error or fatal, and each line names a message and its text,
with FAO directives for arguments. The `MESSAGE` command compiles it
into an object module that is linked into an image or a message-only
shareable image, and can write symbol definitions for the languages.
`$GETMSG` finds a status's text in the image's message sections, the
process's message file (`SET MESSAGE`) and `SYS$MESSAGE`; `$PUTMSG`
writes `%FAC-S-IDENT, text` that way.

vaxpunk has no message files: `$GETMSG` reads one table in the executive
with the file system's, RMS's and some of TCP/IP Services' messages
(`getmsg.mar`); the ponytail says to add message files "when an image
brings its own facility's messages" (PRD-0003 tier 3 #26). WASMRUN's
PRD plans a `WASMMSG.MSG`, and TPU's `%TPU-` messages live in its own
table. A host compiler for `.MSG` files would come first, since the
whole system is built on the host.
