+++
title = "DQS"
summary = "Distributed Queuing Service: print to a queue on another VMS system"
aka = ["DQS", "Distributed Queuing Service", "DQS$SERVER", "DQS$REMOTE_", "client queue", "remote printing"]
kind = "layered"
intent = "aspiration"
research = "deferred"
status = "todo"
refs = ["https://docs.vmssoftware.com/distributed-queuing-service-manager-guide/", "https://docs.vmssoftware.com/distributed-queuing-service-installation-guide/", "https://docs.vmssoftware.com/distributed-queuing-service-user-guide/"]

[[needs]]
on = "//utils/job-controller"
why = "Client queues are ordinary queues; the server enters jobs in its own"

[[needs]]
on = "//utils/print-symbiont"
why = "The client side is a symbiont that sends the file to the server"

[[needs]]
on = "//exec/logical-names"
why = "DQS$REMOTE_queue names the server node and its queue"

[[needs]]
on = "//net/tcpip-qio"
why = "A transport in place of DECnet, which vaxpunk drops"

[[needs]]
fuzzy = "A client-server protocol over TCP in place of DQS's DECnet object"
why = "Sends jobs and queue requests to the server"
+++

DQS, the Distributed Queuing Service, lets a user on one VMS system
`PRINT` to a printer on another. A client queue is an ordinary queue
whose symbiont copies each job to the DQS server on the remote node,
which enters it in a real print queue there; `SHOW QUEUE` and
`DELETE/ENTRY` work across the link, and the server's manager says which
clients and queues are allowed. It runs only over DECnet, Phase IV or V.

On vaxpunk it is an aspiration. DECnet is dropped, so it would need a
TCP transport; LPD ([//net/lpd](../net/lpd.md)) already covers remote
printing over TCP/IP.
