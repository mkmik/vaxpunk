+++
title = "SHOW CLUSTER"
summary = "A live display of a cluster's members, votes, circuits and disks"
aka = ["SHOW CLUSTER", "SHOW CLUSTER/CONTINUOUS", "SCACP", "CLUSTER_CONFIG.COM"]
kind = "core"
intent = "aspiration"
research = "deferred"
status = "todo"
refs = ["https://docs.vmssoftware.com/vsi-openvms-system-management-utilities-reference-manual-volume-ii-m-z/"]

[[needs]]
on = "//exec/cluster"
why = "Shows the connection manager's view of the members and their links"

[[needs]]
on = "//rtl/smg"
why = "/CONTINUOUS repaints a screen of fields and windows"
soft = true

[[needs]]
on = "//rtl/cli"
why = "Its commands: ADD, REMOVE, SET, a display at a time"
+++

`SHOW CLUSTER` displays the state of an OpenVMS Cluster: each member
node, its software version, its votes and the quorum, the circuits and
connections between nodes over the interconnects (CI, DSSI, LAN), and
the disks served across the cluster. With `/CONTINUOUS` it stays on the
screen and refreshes, and commands add or remove fields and classes of
information. `SCACP` manages the System Communications Architecture
ports and circuits underneath, and `CLUSTER_CONFIG.COM` adds and removes
members.

vaxpunk runs on one node: clusters are a non-goal of PRD-0003, and
ADR-0003 keeps one CPU.
