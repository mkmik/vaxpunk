+++
title = "MMS"
summary = "DECset's Module Management System: VMS's make, driven by DESCRIP.MMS"
aka = ["MMS", "Module Management System", "DECset", "DESCRIP.MMS", "make", "MMK"]
kind = "layered"
intent = "aspiration"
research = "outline"
status = "todo"
refs = ["https://docs.vmssoftware.com/vsi-decset-for-openvms-guide-to-the-module-management-system/", "https://docs.vmssoftware.com/vsi-module-management-system-v405-release-notes/"]

[[needs]]
on = "//dcl/spawn"
why = "Action lines are DCL commands run in a subprocess"

[[needs]]
on = "//dcl/symbols"
why = "MMS$STATUS and MMS$SEVEREST_STATUS report results"

[[needs]]
on = "//rms/core"
why = "Revision dates of targets and sources decide what to rebuild"

[[needs]]
on = "//utils/librarian"
why = "Targets and sources can be modules in object and text libraries"

[[needs]]
on = "//layered/cms"
why = "Builds from a CMS class of generations"
soft = true

[[needs]]
on = "//layered/cdd"
why = "Dependencies on dictionary definitions (path^)"
soft = true
+++

MMS, the Module Management System, is DECset's build tool, VMS's `make`.
It reads a description file, `DESCRIP.MMS` by default, of targets, the
sources they depend on and the DCL action lines that make them, plus
macros and built-in rules (`.PAS` to `.OBJ`, `.OBJ` to `.EXE`). It
rebuilds only targets older than their sources, and understands modules
in libraries, CMS generations and CDD definitions as sources. Action
lines run in a DCL subprocess, and their status comes back in symbols.
MMK is a free work-alike that most open-source VMS software uses.

On vaxpunk it is an aspiration. vaxpunk is built on the host by Cargo
and `build.rs`. If vaxpunk comes to build itself, which is a stated
long-term goal, a VMS build tool such as MMS or MMK is how VMS sites
drove such builds.
