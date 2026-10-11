+++
title = "Home-office inventory"
summary = "Marko's signature application: an inventory on ACMS, DECforms, CDD and RMS"
aka = ["signature application", "inventory", "stock control", "VMS alive"]
kind = "layered"
intent = "aspiration"
research = "outline"
status = "todo"

[[needs]]
on = "/layered/acms"
why = "The TP monitor: menus, tasks, servers"

[[needs]]
on = "/layered/acms#exchange-decforms"
why = "Every screen is a DECforms exchange step"

[[needs]]
on = "/layered/decforms"
why = "The forms the user fills in"

[[needs]]
on = "/layered/cdd#records"
why = "The record definitions forms, workspaces and procedures share"

[[needs]]
on = "/rms/indexed"
why = "The master files, read by key; Rdb is out"

[[needs]]
on = "/rms/journaling#recovery-units"
why = "A move or a loan that changes several records is one recovery unit"
soft = true

[[needs]]
fuzzy = "A language for the step procedures: COBOL in DEC's examples, or BLISS or MACRO-32 on vaxpunk"
why = "Server procedures do the file work"
+++

Marko's idea for a "signature application" for a world where VMS is
still alive: the kind of transaction processing program VMS sites ran
every day, rather than a Unix program ported over. It is a home-office
inventory, stock control for the things in an office's drawers: what an
item is, where it is, when it was bought and for how much, its
condition, when it was last tested, whether it works. Its tasks are the
things that happen to an item: acquire, move, lend, mark broken or
repaired, retire. There is no sales side.

It is built the way DEC would have sold it: ACMS tasks and menus,
DECforms screens, record definitions in CDD, and the data in RMS indexed
files. Rdb is explicitly out: too heavy for one user's inventory, and
indexed master files are the older VMS way. It is an aspiration, not in
a PRD. Indexed files come with
[PRD-0008](../../prd/0008-rms-record-and-indexed-files.md); the
rest of the chain is in [ACMS](acms.md) and [DECforms](decforms.md). One
snag is in [CDD](cdd.md): today's CDD/Repository keeps its own
definitions in Rdb.
