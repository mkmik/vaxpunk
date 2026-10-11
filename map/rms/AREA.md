+++
title = "RMS"
band = "data"
summary = "Record Management Services: records, files, sharing"
+++

RMS is how a VMS program reads and writes files: the FAB, RAB, NAM and
XAB control blocks, the services `$OPEN`, `$GET`, `$PUT`, `$SEARCH` and
the rest, file specifications with logical names, defaults and
wildcards, three file organizations (sequential, relative, indexed) and
seven record formats, and the record locks that let processes share a
file. It runs in executive mode in each process, on top of the XQP
(`map/files`). Around it sit FDL, the callable and DCL utilities that
make and check files (`CONVERT`, `ANALYZE/RMS_FILE`, in `map/utils`),
and options: global buffers, RMS Journaling, and remote access over
DECnet.

vaxpunk's RMS is MACRO-32 in the executive, in kernel mode: `rms.mar`
(names, FABs, the services), `rmsseq.mar`, `rmsrel.mar`, `rmsidx.mar`,
`rmsidxw.mar` and `rmsxab.mar`
([ADR-0007](../../docs/adr/0007-system-disk-files-11-and-rms.md),
[PRD-0008](../../docs/prd/0008-rms-record-and-indexed-files.md)). Every
organization and record format is in, bit for bit as OpenVMS writes
them; sharing waits for the lock manager (PRD-0003 step 7).
