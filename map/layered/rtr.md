+++
title = "Reliable Transaction Router"
summary = "Fault-tolerant transactions across nodes: front ends, routers, back ends"
aka = ["RTR", "Reliable Transaction Router", "DECrtr", "facility", "shadow servers", "standby servers"]
kind = "layered"
intent = "aspiration"
research = "deferred"
status = "todo"
refs = ["https://en.wikipedia.org/wiki/HP_RTR", "https://www.manualslib.com/manual/30235/Compaq-Reliable-Transaction-Router.html"]

[[needs]]
on = "//net/tcpip-qio"
why = "Its virtual network runs over TCP/IP (or DECnet)"

[[needs]]
on = "//exec/processes"
why = "The RTR ACP process and the client and server processes"

[[needs]]
on = "//rms/core"
why = "The transaction journal used for recovery"

[[needs]]
on = "//exec/decdtm"
why = "Back-end transactions with the database's resource manager"
soft = true

[[needs]]
on = "//exec/cluster:mscp"
why = "Standby servers share a journal on a cluster's disks"
soft = true

[[needs]]
fuzzy = "Its own two-phase commit, routing by key range, and shadow-site replay"
why = "The product itself"
+++

RTR, the Reliable Transaction Router, began at DEC in Zurich in 1988 for
trading and banking systems that could not stop. An RTR facility is a
set of nodes in three roles: front ends, where client programs run;
routers, which send each transaction to the right back end by its key
range and run the two-phase commit; and back ends, where server programs
do the work against a database. Standby servers take over when a back
end fails, often on another node of a cluster sharing the journal, and
shadow servers at a second site process the same transactions so that
site can carry on alone. The first version was MACRO-32, BLISS and
Pascal on DECnet; it was rewritten in C on TCP/IP and ported to Unix and
Windows.

On vaxpunk it is an aspiration, next to clusters
([//exec/cluster](../exec/cluster.md)), which are a non-goal of
Milestone 1.
