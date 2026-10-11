+++
title = "Xlib"
summary = "The X11 client library: connections, windows, graphics, events"
aka = ["Xlib", "libX11", "DECW$XLIBSHR", "X11", "X Window System library", "XOpenDisplay"]
kind = "layered"
intent = "aspiration"
research = "outline"
refs = ["https://h41379.www4.hpe.com/doc/82final/6470/6470pro_contents_001.html"]

[[feature]]
id = "protocol"
title = "Opens a display and speaks the X11 protocol: windows, graphics, text, events"
status = "todo"
needs = ["/gui/x-transport#set-display", "/gui/x-transport#tcpip"]

[[feature]]
id = "vms-events"
title = "Event waits on event flags and ASTs, VMS's way rather than select()"
status = "todo"
needs = ["/exec/event-flags", "/exec/asts"]
note = "DECwindows's Xlib has VMS extensions for this; to check in its reference"

[[feature]]
id = "extensions"
title = "Extensions and their libraries: SHAPE, XKB, ICE and session management"
status = "todo"

[[feature]]
id = "i18n"
title = "Locales, input methods and the internationalization library (Xnl)"
status = "todo"

[[needs]]
on = "/gui/x-transport"
why = "Reaches the X server through it"

[[needs]]
on = "/exec/calling-standard"
why = "Called from any language by the standard"

[[needs]]
on = "/lang/c"
why = "X.Org's libX11 is C, and a port would come through the C path"
soft = true

[[needs]]
on = "/rtl/crtl"
why = "A port of libX11 calls the C library"
soft = true

[[needs]]
fuzzy = "ICCCM, the resource manager (Xrm), XKB keyboard handling, Xcms colour and font names"
why = "The conventions and sublibraries a client needs besides the protocol"

[[needs]]
fuzzy = "Which language: X.Org's C sources ported, or a rewrite"
why = "Not chosen"
+++

Xlib is the X Window System's C library for clients: it opens a
connection to an X server, creates windows, draws, loads fonts and reads
events, turning calls into X11 protocol requests. DECwindows ships it as
`DECW$XLIBSHR.EXE`, with the X11R6 interfaces, a transport layer behind
`SET DISPLAY` rather than BSD sockets alone, and VMS additions for
waiting on events with event flags and ASTs (to be confirmed in DEC's
Xlib reference). Every other layer of DECwindows sits on it.

vaxpunk has none. It is the first layer Marko's plan needs: X11 clients
on vaxpunk talking to Xvnc on the Mac.
