+++
title = "SMG$ screen management"
summary = "Pasteboards, virtual displays, viewports and virtual keyboards on terminals"
aka = ["SMG$", "SMGSHR", "SMG$CREATE_PASTEBOARD", "SMG$READ_STRING", "TERMTABLE"]
kind = "core"
intent = "aspiration"
research = "outline"
status = "todo"

[[needs]]
on = "//drivers/terminal"
why = "Reads keys and writes escape sequences through the terminal driver"

[[needs]]
on = "//rtl/librtl"
why = "In LIBRTL's family of shareable images, SMGSHR.EXE"

[[needs]]
fuzzy = "TERMTABLE.EXE, the terminal capability database compiled from TERMTABLE.TXT"
why = "Knows how to drive each terminal type"
+++

SMG$ is the run-time library's screen manager, in `SMGSHR.EXE`. A
program creates a pasteboard for a terminal, virtual displays on it,
viewports, menus and a virtual keyboard, and SMG works out the escape
sequences, optimising the updates, from `TERMTABLE`, the terminal
capability database. Many utilities and countless user programs are
built on it.

vaxpunk has none. EDT and TPU drive the screen with their own code,
for VT100-family terminals.
