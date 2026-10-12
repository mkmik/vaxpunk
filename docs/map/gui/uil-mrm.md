+++
title = "UIL and MRM"
summary = "User Interface Language and the Motif Resource Manager: interfaces as data"
aka = ["UIL", "MRM", "UID", "Motif Resource Manager", "User Interface Language", "DECW$MRMLIBSHR12", "DECW$WML", "WML", ".UIL", ".UID"]
kind = "layered"
intent = "aspiration"
research = "outline"
refs = ["https://wasd.vsm.com.au/htbin/helpgate/HELP/UIL"]

[[feature]]
id = "compiler"
title = "The UIL command: compiles a .UIL description into a .UID file"
status = "todo"
needs = ["/dcl/cld", "/rms/core"]

[[feature]]
id = "mrm"
title = "MRM: a program fetches widget trees from UID files at run time"
status = "todo"
needs = ["/gui/motif#widgets"]

[[feature]]
id = "callable"
title = "The callable UIL compiler, for builders such as VUIT"
status = "todo"

[[feature]]
id = "wml"
title = "WML: the widget meta-language that tells UIL a widget set, for DXm or one's own"
status = "todo"

[[needs]]
on = "/gui/motif"
why = "UIL describes Motif widgets, MRM creates them"

[[needs]]
on = "/gui/dxm"
why = "DECwindows's UIL knows the DXm widgets too"
soft = true

[[needs]]
fuzzy = "The UID file format, and UIL's language"
why = "What the compiler writes and MRM reads"
+++

UIL, the User Interface Language, describes a Motif interface apart from
the program's code: its widgets, their resources and the callbacks they
name. The `UIL` command compiles a `.UIL` file into a `.UID` file, and
at run time MRM, the Motif Resource Manager (`DECW$MRMLIBSHR12.EXE`),
reads it and creates the widgets, so the layout changes without
relinking. It came from DEC's XUI toolkit into Motif. A widget
meta-language, WML, tells the compiler about widget sets beyond Motif's.

vaxpunk has none. Marko names UIL and MRM as part of the work.
