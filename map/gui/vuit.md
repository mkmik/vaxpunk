+++
title = "VUIT"
summary = "Visual User Interface Tool: DEC's interactive builder for Motif interfaces"
aka = ["VUIT", "Visual User Interface Tool", "interface builder"]
kind = "layered"
intent = "aspiration"
research = "deferred"
status = "todo"
refs = ["https://foldoc.org/VUIT"]

[[needs]]
on = "//gui/uil-mrm:callable"
why = "Reads and writes UIL"

[[needs]]
on = "//gui/motif"
why = "Shows the interface with real widgets as it is built"

[[needs]]
on = "//gui/dxm"
why = "Builds DXm widgets too"
soft = true
+++

VUIT, the Visual User Interface Tool, was DEC's interactive editor for
Motif interfaces: you place and arrange widgets, set their resources and
name their callbacks on the screen, and it writes UIL for the UIL
compiler and MRM, with stubs for the program's side. It was sold for
DECwindows Motif on VMS and DEC's UNIX systems.

vaxpunk has none. Marko has said maybe a VUIT-like builder.
