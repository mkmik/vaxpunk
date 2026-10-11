+++
title = "Session Manager"
summary = "The Traditional Desktop's login, application menu, security and settings"
aka = ["Session Manager", "DECW$SESSION", "DECW$LOGINOUT", "Start Session", "Pause"]
kind = "layered"
intent = "aspiration"
research = "deferred"
refs = ["https://typewritten.org/Manual/DEC/VMS/5.4/decw$session.html"]

[[feature]]
id = "login"
title = "The login box, and the session it starts"
status = "todo"
needs = ["//utils/loginout"]

[[feature]]
id = "applications"
title = "The Applications menu: DECterm, FileView and the rest"
status = "todo"
needs = ["//dcl/spawn"]

[[feature]]
id = "security"
title = "Customize Security: which node, user and transport may use the display"
status = "todo"

[[feature]]
id = "customize"
title = "Customize Window, Keyboard and Pointer, kept in the user's resource file"
status = "todo"

[[feature]]
id = "pause"
title = "Pause: the screen locked until the password is typed"
status = "todo"

[[feature]]
id = "messages"
title = "The message area: completions, new mail and warnings"
status = "todo"

[[needs]]
on = "//gui/motif"
why = "Its windows are Motif"

[[needs]]
on = "//gui/window-manager"
why = "Starts the window manager the user chose"
soft = true
+++

The Session Manager is the first thing a DECwindows user sees on the
Traditional Desktop: the login box, then a small window whose menus
start applications, pause the session behind a password, end it, and
customize the screen, keyboard, pointer, window manager and security,
the list of nodes, users and transports allowed to display on this
workstation. Its message area shows completions and new mail. The New
Desktop replaces it with CDE's `dtsession` and `dtlogin`; XDM gives X
terminals the same login.

vaxpunk has none.
