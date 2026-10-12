+++
title = "DECterm"
summary = "The DECwindows terminal emulator: a VT340-class terminal in a window"
aka = ["DECterm", "DECW$TERMINAL", "CREATE/TERMINAL", "TWA", "DECW$TERMINAL_MAILBOX", "ReGIS", "sixel"]
kind = "layered"
intent = "aspiration"
research = "deferred"
status = "todo"
refs = ["https://h41379.www4.hpe.com/doc/82final/9996/9996pro_40.html"]

[[needs]]
on = "/gui/motif"
why = "Its window and menus are Motif"

[[needs]]
on = "/drivers/pseudo-terminal"
why = "Each window is a pseudo terminal the process reads and writes"

[[needs]]
on = "/dcl/spawn"
why = "CREATE/TERMINAL runs the window's process as a subprocess"

[[needs]]
on = "/exec/mailboxes"
why = "CREATE/TERMINAL asks the controller through its mailbox"

[[needs]]
on = "/utils/loginout"
why = "A window with no process asks for a login"
soft = true

[[needs]]
fuzzy = "VT320 and VT340 emulation, with ReGIS and sixel graphics"
why = "What it draws"
+++

DECterm is DECwindows's terminal: a window that behaves as a VT340-class
terminal, with ReGIS and sixel graphics, for DCL and any character-cell
program. `CREATE/TERMINAL` makes one: the controller,
`SYS$SYSTEM:DECW$TERMINAL.EXE`, one process per display, opens the
window on a pseudo terminal, and the command's subprocess, or a login
if there is none, runs on it. It has menus for its settings, a scroll
back and printing.

vaxpunk has none. Its character-cell programs, EDT and EVE among them,
would run in it unchanged.
