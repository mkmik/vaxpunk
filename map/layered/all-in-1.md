+++
title = "ALL-IN-1"
summary = "DEC's office system: mail, word processing, file cabinet, calendar, on menus"
aka = ["ALL-IN-1", "CP/OSS", "Office Server", "WPS-PLUS", "File Cabinet Server", "FCS", "TeamLinks", "Message Router", "MAILbus"]
kind = "layered"
intent = "aspiration"
research = "deferred"
status = "todo"
refs = ["https://en.wikipedia.org/wiki/ALL-IN-1"]

[[needs]]
on = "//layered/fms"
why = "Its screens are FMS forms"

[[needs]]
on = "//drivers/terminal"
why = "A menu system for VT terminals"

[[needs]]
on = "//rms/indexed"
why = "File cabinets, mail and calendars are indexed files"

[[needs]]
on = "//utils/mail"
why = "Exchanges mail with VMS MAIL"

[[needs]]
on = "//utils/print-symbiont"
why = "Prints documents"
soft = true

[[needs]]
on = "//layered/vax-notes"
why = "Conferences from the office menus"
soft = true

[[needs]]
fuzzy = "Message Router and the MAILbus, or SMTP in their place, for mail between systems"
why = "Office mail across nodes; DECnet is dropped"

[[needs]]
fuzzy = "WPS-PLUS, its word processor, and its script language"
why = "Most of what users did in it"
+++

ALL-IN-1 is DEC's office automation system, first released in 1982 as
CP/OSS and renamed in 1983. Behind one menu tree on a VT terminal it
gave office workers electronic mail, a word processor (WPS-PLUS),
a file cabinet of folders and documents, calendars and time management,
and links to VAX Notes, all customizable by a script language and new
forms. Mail between systems went through the Message Router and the
MAILbus. Version 3 added a shared File Cabinet Server for PC clients
(TeamLinks). It was a large business for DEC; it was later sold as
Office Server and is now discontinued.

On vaxpunk it is an aspiration. Its world, office work on a shared VMS
box, is close to the mail and chat between office VMS systems that
[PRD-0002](../../docs/prd/0002-networking.md) wants to unblock, but
nothing chooses it.
