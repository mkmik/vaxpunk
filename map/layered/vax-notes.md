+++
title = "VAX Notes"
summary = "Computer conferencing: conferences of topics and replies, local or remote"
aka = ["VAX Notes", "DEC Notes", "NOTES", "notesfile", "notebook", "conference"]
kind = "layered"
intent = "aspiration"
research = "deferred"
status = "todo"
refs = ["https://openbookproject.net/tutorials/kjcole/vax-notes.html", "https://ftp.math.utah.edu/pub/tex/bib/idx/dectechj/1/6/117_124.html"]

[[needs]]
on = "//rms/indexed"
why = "Conferences and each user's notebook are files read by key"

[[needs]]
on = "//rms/sharing"
why = "Many users read and write one conference at once"

[[needs]]
on = "//drivers/terminal"
why = "An interactive reader at a terminal"

[[needs]]
on = "//net/tcpip-qio"
why = "Remote conferences on other systems, over TCP since DECnet is dropped"
soft = true

[[needs]]
fuzzy = "A conference server and its protocol: a DECnet object on VMS; IANA lists dec-notes on port 3333"
why = "Reading conferences on other nodes"
+++

VAX Notes is DEC's conferencing system from the mid-1980s. A conference
is a file of topics, each with its replies, stored once and read by
everyone who subscribes, unlike mail. Each user's notebook lists the
conferences they follow and what they have seen, so `NEXT UNSEEN` walks
through what is new; moderators manage a conference, and keywords
group its topics. Conferences could live on other nodes and be read
through a multithreaded Notes server over DECnet. Inside DEC thousands
of conferences carried the company's technical discussion, and DECUServe
ran public ones.

On vaxpunk it is an aspiration. It is the kind of chat between office
VMS boxes [PRD-0002](../../docs/prd/0002-networking.md) wants to unblock;
remote conferences would need a TCP transport of their own.
