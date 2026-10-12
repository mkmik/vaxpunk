+++
title = "DECintact"
summary = "DEC's other TP monitor: a 3GL call library with its own recoverable queues"
aka = ["DECintact", "TP monitor"]
kind = "layered"
intent = "aspiration"
research = "deferred"
status = "todo"
refs = ["https://ftp.math.utah.edu/pub/tex/bib/idx/dectechj/3/1/18_32.html", "https://ftp.mirrorservice.org/sites/www.bitsavers.org/pdf/dec/dtj/dtj_v03-01_1991.pdf"]

[[needs]]
on = "/exec/decdtm"
why = "Its transactions sit on DECdtm"

[[needs]]
on = "/exec/processes"
why = "Terminal manager, dispatcher and server processes"

[[needs]]
on = "/exec/lock-manager"
why = "Thread switches happen on lock requests; shared files"

[[needs]]
on = "/rms/indexed"
why = "Its resource manager recovers RMS files"

[[needs]]
on = "/drivers/terminal"
why = "One process serves many terminal users"

[[needs]]
fuzzy = "Its own forms, recoverable queuing subsystem and thread scheduler"
why = "The product itself"
+++

DECintact was DEC's second transaction processing monitor for VMS, sold
next to ACMS. Where ACMS has a fourth-generation task definition
language, DECintact is a library of calls from third-generation
languages; it threads many users inside one process, switching on I/O
and lock requests, hands database work to single-threaded servers, and
has its own recoverable message queues and a resource manager that
recovers RMS files. Remote applications talk through DECnet links
between its terminal managers.

On vaxpunk it is an aspiration. Marko chose ACMS over it for his
signature application idea, as the more mainstream DEC choice.
