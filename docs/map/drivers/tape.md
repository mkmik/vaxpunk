+++
title = "Tape"
summary = "MKA0: and friends: magnetic tapes, ANSI labels, BACKUP to tape"
aka = ["MKA0:", "MUA0:", "MKDRIVER", "TUDRIVER", "MTAACP", "magnetic tape", "ANSI labels", "IO$_REWIND", "IO$_SKIPFILE", "IO$_WRITEOF", "MOUNT/FOREIGN", "TK50", "DLT", "LTO"]
kind = "core"
intent = "aspiration"
research = "outline"
status = "todo"
refs = ["https://docs.vmssoftware.com/vsi-openvms-io-user-s-reference-manual/"]

[[needs]]
on = "/exec/qio"
why = "Tape functions are $QIO requests"

[[needs]]
on = "/drivers/interrupt-io"
why = "A rewind or a skip takes minutes; a polled driver would hold the CPU"

[[needs]]
fuzzy = "A tape device to drive: a virtual tape on a host file, or virtio-scsi to a tape"
why = "QEMU's virt machine has no tape"

[[needs]]
fuzzy = "MTAACP: ANSI-labelled volumes, files on tape through RMS"
why = "INITIALIZE, MOUNT and COPY of tape files"
+++

VMS's tape drivers (`MKDRIVER` for SCSI tapes such as the TK50, DLT
and LTO, `TUDRIVER` for MSCP ones on `MUA0:`) read and write blocks
and position the tape: `IO$_REWIND`, `IO$_SKIPFILE`, `IO$_SKIPRECORD`,
`IO$_WRITEOF` for tape marks, `IO$_REWINDOFF` to unload. Tapes are
mounted as ANSI-labelled volumes, whose files the tape ACP, `MTAACP`,
finds for RMS, or `/FOREIGN` for programs that do their own I/O. BACKUP
save sets on tape were how systems were backed up and how VMS itself
was installed before CD-ROMs.

vaxpunk has no tape. Its BACKUP writes save sets to disk files, which
VMS's BACKUP reads, so a tape is not needed to move files.
