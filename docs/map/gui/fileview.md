+++
title = "FileView"
summary = "The Traditional Desktop's file manager: directories as windows, verbs as DCL"
aka = ["FileView", "VUE", "VUE$MASTER", "DECW$FILEVIEW"]
kind = "layered"
intent = "aspiration"
research = "deferred"
status = "todo"
refs = ["https://uprrp2.uprrp.edu/HyperReader/sys$common/syshlp/DECW$VUE.DECW$BOOK?chunk=21"]

[[needs]]
on = "/gui/motif"
why = "Its windows are Motif"

[[needs]]
on = "/rms/wildcards"
why = "Lists directories with wildcards"

[[needs]]
on = "/dcl/spawn"
why = "Runs the DCL command behind each verb in a subprocess"

[[needs]]
fuzzy = "FileView's verbs and menus, and how users and managers define them"
why = "What it offers on a selected file"
+++

FileView was the Traditional Desktop's graphical face of VMS: a window
on a directory, where you select files with the mouse and choose a
verb, copy, delete, rename, print, edit, run, each carried out as a DCL
command, and can open a window to type DCL commands too. It runs as
`SYS$SYSTEM:VUE$MASTER.EXE`. The New Desktop's File Manager, `dtfile`,
replaced it.

vaxpunk has none.
