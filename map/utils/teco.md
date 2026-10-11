+++
title = "TECO"
summary = "The Text Editor and Corrector, EDIT/TECO: a character editor and macro language"
aka = ["TECO", "EDIT/TECO", "TECO.INI"]
kind = "core"
intent = "aspiration"
research = "deferred"
refs = ["https://h41379.www4.hpe.com/doc/84final/9996/9996pro_72.html"]

[[feature]]
id = "editor"
title = "EDIT/TECO: TECO's commands on a text buffer, with its Q-registers and macros"
status = "todo"
needs = ["//rms/sequential", "//drivers/terminal"]

[[feature]]
id = "video"
title = "Video mode on a VT100, and TECO.INI startup files"
status = "todo"
needs = [":editor"]

[[needs]]
on = "//rms/sequential"
why = "Reads and writes the file"

[[needs]]
on = "//drivers/terminal"
why = "Reads commands a character at a time"
+++

TECO, the Text Editor and Corrector, came to VMS from the PDP-10 and
PDP-11 operating systems. Its commands are single characters that
search, move, insert and delete in a buffer, and string together into
macros kept in Q-registers, which makes it a programming language as
much as an editor; the first Emacs was a set of TECO macros. VMS ships
it as `EDIT/TECO`, still in the OpenVMS V8.4 DCL dictionary, for the
users who never left it.

vaxpunk has none, and nothing plans one: `EDIT` is EVE, and `EDIT/EDT`
is EDT.
