+++
title = "DSM"
summary = "DEC Standard MUMPS: a language and database in one, for hospitals"
aka = ["DSM", "VAX DSM", "DEC Standard MUMPS", "MUMPS", "M", "GT.M"]
kind = "layered"
intent = "aspiration"
research = "deferred"
status = "todo"

[[needs]]
on = "//exec/lock-manager"
why = "MUMPS's LOCK command and shared globals among many users"

[[needs]]
on = "//drivers/terminal"
why = "Its applications are terminal applications for many users"

[[needs]]
fuzzy = "A MUMPS interpreter or compiler, and its database of globals on disk with journaling"
why = "The language and its storage"
+++

MUMPS is a language with its own database built in: variables whose
names start with `^` are globals, sparse trees kept on disk and shared
by every user, with `LOCK` and transactions. Hospitals and laboratories
ran their systems on it. DEC's implementation, DSM, ran on the PDP-11
and on VMS as VAX DSM, and later went to InterSystems. Greystone's
GT.M, another MUMPS, also started on VMS.

vaxpunk has none.
