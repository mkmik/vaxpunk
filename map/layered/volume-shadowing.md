+++
title = "Volume Shadowing"
summary = "Host-based disk mirroring: shadow sets of members behind a DSAn: unit"
aka = ["Volume Shadowing for OpenVMS", "HBVS", "host-based volume shadowing", "shadow set", "DSA0:", "SHDRIVER", "minimerge", "minicopy", "write bitmaps", "RAID 1"]
kind = "layered"
intent = "aspiration"
research = "outline"
refs = ["https://docs.vmssoftware.com/vsi-openvms-volume-shadowing-guide/"]

[[feature]]
id = "shadow-sets"
title = "Shadow sets: MOUNT a DSAn: virtual unit over two or three member disks"
status = "todo"
needs = ["//utils/mount", "//drivers/disk"]

[[feature]]
id = "copy-merge"
title = "Copy to a new member, merge after a failure"
status = "todo"
needs = [":shadow-sets"]

[[feature]]
id = "system-disk"
title = "A shadowed system disk"
status = "todo"
needs = [":shadow-sets", "//exec/system-init"]

[[feature]]
id = "bitmaps"
title = "Write bitmaps: minicopy and host-based minimerge"
status = "todo"
needs = [":copy-merge"]

[[feature]]
id = "cluster-wide"
title = "Cluster-wide shadow sets, members served by MSCP from other nodes"
status = "todo"
needs = [":shadow-sets", "//exec/cluster:mscp"]

[[feature]]
id = "six-members"
title = "Six members per set, for sites far apart (V8.4)"
status = "todo"
needs = [":cluster-wide"]

[[needs]]
on = "//exec/qio"
why = "The virtual unit is a driver that sends each write to every member"

[[needs]]
on = "//drivers/disk"
why = "Members are ordinary disks"

[[needs]]
on = "//files/volumes"
why = "A shadow set is mounted as one volume"

[[needs]]
on = "//exec/lock-manager"
why = "Nodes agree on membership and on merges"

[[needs]]
on = "//exec/sysgen-parameters"
why = "SHADOWING, ALLOCLASS and the SHADOW_ parameters turn it on"

[[needs]]
on = "//exec/cluster"
why = "Most of its point is a cluster sharing members across nodes"
soft = true
+++

Volume Shadowing for OpenVMS is RAID 1 done by the host: two or three
disks, the members, are mounted as one virtual unit, `DSAn:`, and every
write goes to all of them, while reads are spread across them. A new or
returning member is brought up to date by a copy; after a crash, a
merge makes the members agree again. Write bitmaps cut both down to the
blocks that changed (minicopy, minimerge), and V8.4 raised the limit to
six members for sites far apart. Members may sit behind different nodes
of a cluster and be reached through MSCP, so it is the base of VMS's
disaster-tolerant clusters. It is licensed separately.

On vaxpunk it is an aspiration, cluster-adjacent; clusters are a
non-goal of Milestone 1. A single-node shadow set of virtio disks would
need only a pseudo-device driver over the disk driver.
