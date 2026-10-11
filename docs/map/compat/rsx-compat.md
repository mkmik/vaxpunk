+++
title = "RSX-11 compatibility mode"
summary = "VAX's PDP-11 compatibility mode: RSX-11M programs under VAX/VMS"
aka = ["VAX-11 RSX", "compatibility mode", "AME", "MCR", "RSX-11M"]
kind = "layered"
intent = "aspiration"
research = "deferred"
status = "todo"

[[needs]]
on = "/exec/processes"
why = "An RSX task ran inside a VMS process"

[[needs]]
fuzzy = "A PDP-11 instruction set emulator or translator, and the RSX-11M executive directives"
why = "The Alpha had no compatibility mode, so this would be software"
+++

Early VAX processors could run PDP-11 code in compatibility mode, and
VAX/VMS used it to run RSX-11M tasks: the Application Migration
Executive turned RSX directives into VMS services, and some early VMS
utilities ran this way. VAX-11 RSX, the layered
product, kept the RSX environment for customers. Alpha VMS dropped it,
and DCL's `MCR` command is all that remains.

vaxpunk follows Alpha VMS and has none.
