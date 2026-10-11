+++
title = "Volume sets"
summary = "Several disks bound into one Files-11 volume, with MOUNT/BIND"
aka = ["bound volume set", "MOUNT/BIND", "VOLSET.SYS", "RVN", "relative volume number"]
kind = "core"
intent = "aspiration"
research = "outline"
refs = ["https://wasd.vsm.com.au/htbin/helpgate/HELP/MOUNT/QUALIFIERS", "https://de.openvms.org/TUD2010/File_System_Changes.pdf", "crosstools/ods/docs/home-block.md", "crosstools/ods/README.md"]

[[feature]]
id = "bind"
title = "MOUNT/BIND makes a volume set, or adds a volume to one; VOLSET.SYS lists them"
status = "todo"
needs = ["/files/volumes#more-disks"]

[[feature]]
id = "rvn"
title = "File IDs with a relative volume number; headers and blocks on any volume"
status = "todo"
needs = ["#bind", "/files/ods2#extension-headers"]

[[feature]]
id = "mount-set"
title = "MOUNT of the set's volumes together, the MFD on the first"
status = "todo"
needs = ["#bind"]

[[needs]]
on = "/files/ods2"
why = "Each volume is an ODS-2 volume with its own index file and bitmap"

[[needs]]
on = "/files/volumes"
why = "The set is mounted as one volume, under one name"
+++

A bound volume set is up to 255 Files-11 volumes that act as one: each
keeps its own index file and storage bitmap, the MFD lives on the first,
and `VOLSET.SYS` there lists the members. A file ID's third part, the
relative volume number, says which volume a header is on, and a file's
extension headers and blocks may be on any of them, so a file can be
bigger than a disk. `MOUNT/BIND=name` makes the set or adds a volume to
it. Volume sets let VMS hold more than one disk's worth in a directory
tree; with disks of 1 TB and more, and logical disks, few sites still
build them.

vaxpunk's volumes are one disk each, and every file ID's relative volume
number is 0. The host's `ods` doesn't do volume sets either.
