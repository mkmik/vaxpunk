+++
title = "EXCHANGE"
summary = "Reads and writes foreign volumes: RT-11 and DOS-11 disks and tapes"
aka = ["EXCHANGE", "EXCHANGE/NETWORK", "RT-11", "DOS-11", "virtual device"]
kind = "core"
intent = "aspiration"
research = "deferred"
status = "todo"
refs = ["https://wasd.vsm.com.au/htbin/helpgate/HELP/EXCHANGE"]

[[needs]]
on = "/utils/mount"
why = "The volume is mounted /FOREIGN, or is a Files-11 file that holds an image of one"

[[needs]]
on = "/rms/core"
why = "Converts files to and from RMS files"

[[needs]]
fuzzy = "RT-11 and DOS-11 volume formats"
why = "The structures it reads and writes"
+++

`EXCHANGE` moves files between Files-11 and volumes in formats VMS
doesn't mount: RT-11 disks and DECtapes, DOS-11 tapes, and the console
media of early VAXes. It has its own prompt and commands (`COPY`,
`DIRECTORY`, `INITIALIZE`, `MOUNT`, `DELETE`, `RENAME`, `TYPE`), converts
record formats as it copies, and works on virtual devices, Files-11
files that hold a whole foreign volume. `EXCHANGE/NETWORK` copies files
to systems that don't understand RMS organizations, choosing block or
record transfer.

vaxpunk has nothing like it. It belongs with the PDP-11 heritage that
vaxpunk, modelled on Alpha VMS, doesn't need.
