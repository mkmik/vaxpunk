+++
title = "Magnetic tape volumes"
summary = "ANSI-labelled tapes, mounted and read through RMS, served by MTAACP"
aka = ["MTAACP", "ANSI labels", "VOL1", "HDR1", "HDR2", "MOUNT tape", "INITIALIZE tape", "continuation volume"]
kind = "core"
intent = "aspiration"
research = "deferred"
status = "todo"

[[needs]]
on = "//drivers/tape"
why = "Reads and writes the tape's blocks and marks"

[[needs]]
on = "//files/volumes"
why = "MOUNT and INITIALIZE of a tape, by its VOL1 label"

[[needs]]
on = "//rms/sequential"
why = "Files on a tape are sequential files"
+++

VMS writes tapes in the ANSI standard's labelled format: a `VOL1` label
names the volume, and each file has `HDR1` and `HDR2` labels before it and
end-of-file labels after, which hold its name and record format. `MOUNT`
and `INITIALIZE` take a tape like a disk, and RMS reads and writes its
files sequentially; a file can continue on the next reel. Unlike disks,
tapes kept an ancillary control process, `MTAACP`, after V4.
`MOUNT/FOREIGN` gives a program the tape without labels, and `BACKUP`
writes its save sets either way.

vaxpunk has no tape drive and nothing planned for one; `BACKUP` writes
its save sets to disk files.
