+++
title = "Availability Manager"
summary = "Watches and fixes VMS nodes in real time over a LAN protocol of its own"
aka = ["Availability Manager", "AM", "DECamds", "AMDS", "Data Collector", "Data Analyzer", "AMDS$DRIVER_ACCESS.DAT", "RMDRIVER"]
kind = "layered"
intent = "aspiration"
research = "deferred"
status = "todo"
refs = ["https://docs.vmssoftware.com/vsi-availability-manager-user-s-guide/", "https://space.physics.uiowa.edu/pi/docs/vms83/opsys/vmsos831h1/6552/6552pro_001.html"]

[[needs]]
on = "//exec/information-services"
why = "CPU, memory, process and disk data, like $GETJPI and $GETSYI"

[[needs]]
on = "//drivers/network"
why = "Talks over its own Ethernet protocol, not TCP/IP"

[[needs]]
on = "//exec/lock-manager"
why = "Reports lock contention and blocking locks"

[[needs]]
on = "//utils/opcom"
why = "Denied and write operations are logged to OPCOM"

[[needs]]
on = "//net/tcpip-qio"
why = "The Data Server relays to analyzers over IP and TLS"
soft = true

[[needs]]
fuzzy = "A collector in the executive that answers at high IPL, even when the node is hung"
why = "Fixes on a hung node: process priorities, quotas, working sets, crashing the node"
+++

The Availability Manager, first sold as DECamds, watches a group of VMS
nodes from a console. A Data Collector in each node's executive answers
an analyzer over an Ethernet protocol of its own, not TCP/IP, so it
works when the network stack or the node is hung. The Data Analyzer, a
Java program on VMS or Windows, shows CPU, memory, process, disk, lock
contention and cluster data, posts events when thresholds are crossed,
and can apply fixes: change a process's priority or quotas, trim working
sets, or crash a hung node. Access is guarded by passwords per address
in `AMDS$DRIVER_ACCESS.DAT`. VSI ships it for x86 too.

On vaxpunk it is an aspiration. Nothing needs it.
