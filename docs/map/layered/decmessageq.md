+++
title = "DECmessageQ"
summary = "Message queuing between programs: groups, queues, PAMS calls"
aka = ["DECmessageQ", "BEA MessageQ", "MessageQ", "PAMS", "pams_put_msg", "MRS"]
kind = "layered"
intent = "aspiration"
research = "deferred"
status = "todo"
refs = ["https://docs.oracle.com/cd/E12531_01/msgq50/intro/i_chap2.htm", "https://docs.oracle.com/cd/E12531_01/msgq50/vmsconf/chap4.htm"]

[[needs]]
on = "/exec/global-sections"
why = "Queues of a group in shared memory, likely"

[[needs]]
on = "/exec/asts"
why = "pams_get_msga delivers messages by AST on VMS"

[[needs]]
on = "/exec/processes"
why = "Each group runs server processes"

[[needs]]
on = "/net/tcpip-qio"
why = "Links between groups on different systems"

[[needs]]
on = "/rms/core"
why = "Recoverable messages are journalled to disk"

[[needs]]
fuzzy = "How a group's queues and servers are built on VMS: shared memory, mailboxes or locks"
why = "Not in the documents read"
+++

DECmessageQ is DEC's message-oriented middleware, later BEA MessageQ.
Programs attach to queues in a group, one group per system, and groups
on different systems join into a message queuing bus. A program sends
with `pams_put_msg` and receives with `pams_get_msg`, or on VMS by AST
with `pams_get_msga`; queues are temporary or permanent, single- or
multi-reader. Recoverable delivery modes keep a message in a disk
journal until the receiver confirms it, with dead-letter queues for
what can't be delivered.

On vaxpunk it is an aspiration. Nothing needs it.
