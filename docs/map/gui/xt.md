+++
title = "X Toolkit Intrinsics"
summary = "Xt: widgets, resources, callbacks and the event loop that toolkits build on"
aka = ["Xt", "Intrinsics", "X Toolkit", "libXt", "DECW$XTLIBSHRR5", "DECW$XTSHR", "XtAppMainLoop", "app-defaults"]
kind = "layered"
intent = "aspiration"
research = "outline"

[[feature]]
id = "widgets"
title = "Widget classes, resources, callbacks, translations and the main loop"
status = "todo"
needs = ["/gui/xlib#protocol"]

[[feature]]
id = "vms-input"
title = "Other input in the main loop: an event flag and an IOSB, VMS's way"
status = "todo"
needs = ["/gui/xlib#vms-events"]
note = "On VMS XtAppAddInput takes an event flag and an IOSB; to check in DECwindows's Xt reference"

[[feature]]
id = "timers"
title = "Timeouts in the main loop"
status = "todo"
needs = ["/exec/timers"]

[[feature]]
id = "resource-files"
title = "Resource files: DECW$USER_DEFAULTS, DECW$SYSTEM_DEFAULTS and application defaults"
status = "todo"
needs = ["/rms/core", "/exec/logical-names"]

[[needs]]
on = "/gui/xlib"
why = "Every widget is an X window"

[[needs]]
fuzzy = "Session management (XSMP over ICE) for the shell widgets"
why = "Saving and restoring a session's applications"
+++

The X Toolkit Intrinsics, Xt, are the layer between Xlib and a widget
set: widget classes and instances, resources read from resource files,
callbacks, translation tables from events to actions, and the main loop
that dispatches events, timeouts and other input. Motif is built on
them. DECwindows ships them as a shareable image,
`DECW$XTLIBSHRR5.EXE`, and reads resource files from the user's and the
system's DECwindows defaults directories.

vaxpunk has none.
