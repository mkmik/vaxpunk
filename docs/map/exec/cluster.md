+++
title = "OpenVMS Cluster"
summary = "Several VMS nodes sharing disks, locks, queues and users as one system"
aka = ["VMScluster", "VAXcluster", "OpenVMS Cluster", "SCS", "System Communications Services", "connection manager", "quorum", "VOTES", "EXPECTED_VOTES", "quorum disk", "MSCP server", "TMSCP", "PEDRIVER", "cluster over IP", "distributed lock manager", "LNM$CLUSTER", "SHOW CLUSTER", "CLUSTER_CONFIG.COM", "common system disk", "$ICC", "CLUEXIT"]
kind = "core"
intent = "aspiration"
research = "outline"
refs = ["docs/prd/0003-multi-user-vms.md", "docs/adr/0003-one-cpu-many-threads.md", "docs/adr/0030-lock-manager-in-the-executive.md", "docs/boot.md", "vms/exec/getjpi.mar"]

[[feature]]
id = "scs"
title = "SCS: connections and messages between nodes over a port driver"
status = "todo"

[[feature]]
id = "interconnect-ip"
title = "The cluster interconnect over IP, PEDRIVER on UDP, as VMS V8.4 added"
status = "todo"
needs = ["#scs", "/net/tcpip-qio"]

[[feature]]
id = "connection-manager"
title = "The connection manager: membership, state transitions, a node leaving with CLUEXIT"
status = "todo"
needs = ["#scs"]

[[feature]]
id = "quorum"
title = "Quorum: VOTES, EXPECTED_VOTES and a quorum disk keep a partitioned cluster from splitting"
status = "todo"
needs = ["#connection-manager"]

[[feature]]
id = "dlm"
title = "The lock manager's remote side: a resource directory, masters on any node, remastering"
status = "todo"
needs = ["#connection-manager", "/exec/lock-manager"]
note = "ADR-0030: the interface is the same, the remote side goes under $ENQ"

[[feature]]
id = "shared-files"
title = "Files-11 and RMS shared across nodes through the lock manager"
status = "todo"
needs = ["#dlm", "/files/fs-locking", "/rms/sharing"]

[[feature]]
id = "mscp"
title = "The MSCP and TMSCP servers: a node's disks and tapes served to the others"
status = "todo"
needs = ["#scs"]

[[feature]]
id = "common-system-disk"
title = "A common system disk: each node's root, SYS$SPECIFIC, beside SYS$COMMON"
status = "todo"
needs = ["/exec/system-init#rooted-directories"]
note = "SYSTARTUP_VMS.COM already makes SYS$SYSROOT such a search list, with the ramdisk as SYS$SPECIFIC"

[[feature]]
id = "cluster-names"
title = "Cluster-wide logical names, LNM$CLUSTER, and cluster-wide $GETSYI and $GETJPI"
status = "todo"
needs = ["#connection-manager"]

[[feature]]
id = "shared-security"
title = "One SYSUAF, RIGHTSLIST, intrusion database and audit policy for every node"
status = "todo"
needs = ["#shared-files"]

[[feature]]
id = "queues"
title = "Cluster-wide queues: one queue manager, jobs running on any node"
status = "todo"
needs = ["#shared-files"]

[[feature]]
id = "management"
title = "SHOW CLUSTER, SYSMAN's cluster environment, CLUSTER_CONFIG.COM"
status = "todo"
needs = ["#connection-manager"]

[[feature]]
id = "icc"
title = "Intra-cluster communication, $ICC_OPEN_ASSOC and friends"
status = "todo"
needs = ["#scs"]

[[needs]]
on = "/exec/lock-manager"
why = "Everything shared across nodes is coordinated with locks"

[[needs]]
on = "/net/tcpip-qio"
why = "vaxpunk's only network; the interconnect would run over IP"
soft = true

[[needs]]
on = "/layered/volume-shadowing"
why = "Shadow sets with members on several nodes"
soft = true

[[needs]]
fuzzy = "A cluster interconnect: SCS's message and block transfer protocol, and PEDRIVER's on Ethernet or UDP"
why = "Nodes exchange lock, membership and disk traffic over it"

[[needs]]
fuzzy = "The connection manager's membership and quorum algorithm, and cluster state transitions"
why = "Decides who is in the cluster and stops a partition writing disks"

[[needs]]
fuzzy = "Several vaxpunk nodes to test on: QEMUs on a shared network, sharing disks or serving them"
why = "Nothing can be checked on one node"

[[needs]]
fuzzy = "Cluster-wide time and per-node system parameters"
why = "Lock timeouts, quorum and logs need agreeing clocks; each node has its own SCSNODE and votes"
+++

An OpenVMS Cluster, a VAXcluster from VMS V4 in 1984, makes several
machines one system. Nodes talk through System Communications Services
over an interconnect: CI, DSSI, Ethernet (`PEDRIVER`), MEMORY CHANNEL,
and from V8.4 IP. The connection manager decides which nodes are
members, and quorum (each node's `VOTES` against `EXPECTED_VOTES`,
with an optional quorum disk) stops a cluster split in two from
writing the same disks. The distributed lock manager coordinates
everything shared: Files-11 volumes mounted on every node at once,
RMS files open on several nodes, the queue manager, cluster-wide
logical names. The MSCP and TMSCP servers let a node serve its disks
and tapes to the rest, and volume shadowing mirrors disks across
nodes. Users see one system: one UAF, one rights database, one set of
queues, a common system disk with a root per node, and logins on any
node. Clusters ran for years without a full outage, and are what VMS
was known for.

vaxpunk is one node with one CPU
([ADR-0003](../../adr/0003-one-cpu-many-threads.md)), and
clusters are a non-goal of PRD-0003, not dropped. A few pieces are
shaped for them: the lock manager is the single-node half of VMS's,
with the same interface, so the remote side can go under `$ENQ` as on
VMS ([ADR-0030](../../adr/0030-lock-manager-in-the-executive.md));
`SYSTARTUP_VMS.COM` makes `SYS$SYSROOT` a search list of a
node-specific root and a common one, as a common system disk has it;
`$GETSYI` ignores its node arguments. vaxpunk networks only over
TCP/IP, so an interconnect would be VMS V8.4's cluster over IP.
