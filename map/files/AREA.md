+++
title = "Files-11"
band = "data"
summary = "The on-disk file system and volumes"
+++

Files-11 is VMS's disk file system: the on-disk structure (ODS-2, and
ODS-5 for long names), the executive code that reads and writes it (the
XQP), volumes and how they are mounted, and the locks and caches around
them. RMS, in `map/rms`, sits on top and gives programs records and file
names. Other structures VMS mounts, CD-ROMs and labelled tapes, live here
too.

In vaxpunk the XQP is `vms/exec/f11.mar` and `f11wrt.mar`, which RMS calls
directly rather than through `$QIO`. The host-side twin, which builds the
system disk and checks volumes, is `crosstools/ods` (`//host/ods`).
