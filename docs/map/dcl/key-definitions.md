+++
title = "Key definitions"
summary = "DEFINE/KEY: keypad and function keys as DCL commands, with states"
aka = ["DEFINE/KEY", "SET KEY", "SHOW KEY", "DELETE/KEY", "keypad", "SET TERMINAL/APPLICATION_KEYPAD", "/IF_STATE", "/SET_STATE"]
kind = "core"
intent = "aspiration"
research = "outline"
status = "todo"

[[needs]]
on = "/dcl/interpreter"
why = "DCL reads the key and puts its text on the line"

[[needs]]
on = "/drivers/terminal"
why = "Escape sequences end the read; the keypad in application mode"

[[needs]]
on = "/dcl/recall"
why = "DCL's own read of the line, which keys and recall share"
soft = true
+++

`DEFINE/KEY PF1 "SHOW TIME"` makes a keypad or function key type a
string at DCL's prompt, and with `/TERMINATE` send it too. Keys have
states: `/SET_STATE=GOLD` changes the state, and `/IF_STATE=GOLD` makes
a definition apply only in it, so a site can give the keypad an
EDT-like layout of commands. `SET KEY/STATE` changes the state by hand,
`SHOW KEY` lists the definitions, and `SET TERMINAL/APPLICATION_KEYPAD`
makes the keypad send escape sequences DCL can tell apart. Definitions
are per process and usually made in `LOGIN.COM`.

Nothing is built and nothing asks for it. DCL reads lines through the
terminal driver's line editing, which drops the escape sequences of keys
it doesn't use.
