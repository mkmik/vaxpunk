+++
title = "DECnet"
summary = "DEC's own network: DECnet Phase IV and DECnet-Plus (Phase V)"
aka = ["DECnet", "DECnet-VAX", "DECnet-Plus", "Phase IV", "Phase V", "NCP", "NCL", "CTERM", "DAP", "FAL", "DECnet over IP", "PWIP"]
kind = "layered"
intent = "aspiration"
research = "deferred"
status = "dropped"
refs = ["docs/prd/0002-networking.md"]
+++

DECnet was the network VMS grew up with: Phase IV, with 16-bit node
addresses, managed with `NCP`, and DECnet-Plus, Phase V, OSI-based,
managed with `NCL`. It gave VMS `node::` file names everywhere through
DAP and FAL, `SET HOST` through CTERM, task-to-task links, MAIL's
`NODE::USER`, PHONE, proxies, and cluster communication. DECnet-Plus
can run over TCP/IP through TCP/IP Services' PWIP driver.

Dropped in PRD-0002: "Not interesting to rebuild; for DECnet
archaeology, run a real copy of VMS". vaxpunk does TCP/IP only, and
takes DECnet's applications, remote login, remote files, mail and chat,
over to TCP/IP.
