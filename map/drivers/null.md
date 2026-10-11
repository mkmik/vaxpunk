+++
title = "Null device"
summary = "NLA0:, NL:: writes vanish, reads see end of file at once"
aka = ["NLA0:", "NL:", "NLDRIVER", "_NLA0:", "null device"]
kind = "core"
intent = "aspiration"
research = "done"
status = "todo"

[[needs]]
on = "//exec/qio"
why = "An FDT routine that completes every request at once"
+++

`NLA0:`, usually written `NL:`, is VMS's null device. Writes succeed and
go nowhere; reads end with an end of file at once. Procedures send
output they don't want there, with `DEFINE/USER SYS$OUTPUT NL:` before a
command or `/OUTPUT=NL:`; programs open it for an empty input; `COPY
NL: file` makes an empty file; detached processes are often created
with it as their input and output. It is a record device
RMS opens like a file.

vaxpunk has none. A driver is one FDT routine and a UCB in `qio.mar`'s
list, and RMS would need to open a record device that isn't a disk or a
terminal.
