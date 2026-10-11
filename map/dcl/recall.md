+++
title = "Line recall"
summary = "Earlier commands brought back with the arrows, and the RECALL command"
aka = ["RECALL", "up arrow", "CTRL/B", "RECALL/ALL", "RECALL/SEARCH", "recall buffer"]
kind = "core"
intent = "goal"
research = "done"
refs = ["vms/exec/ttdriver.mar", "docs/adr/0027-terminals-are-ucbs-and-telnet-is-in-the-driver.md", "docs/design/0002-executive-processes.md", "docs/prd/0003-multi-user-vms.md", "https://h41379.www4.hpe.com/doc/84final/9996/9996pro_161.html"]

[[feature]]
id = "terminal-recall"
title = "Up arrow or CTRL/B and down arrow recall the last 16 lines, from the terminal's buffer"
status = "done"
milestone = "m0"
note = "ttdriver.mar; each terminal's own since ADR-0027"

[[feature]]
id = "dcl-buffer"
title = "DCL's own recall buffer, so an image's input doesn't mix with commands"
status = "todo"
needs = ["//drivers/terminal:itemlist-read"]
note = "PRD-0003 tier 3 #29; ADR-0027 follow-up"

[[feature]]
id = "recall-command"
title = "RECALL [n | string], /ALL, /ERASE, /SEARCH"
status = "todo"
needs = [":dcl-buffer"]
note = "PRD-0003 tier 3 #29"

[[feature]]
id = "recall-files"
title = "RECALL/INPUT and /OUTPUT: the buffer from and to a file"
status = "todo"
needs = [":recall-command"]

[[needs]]
on = "//drivers/terminal"
why = "The arrows are read and the line redisplayed by the terminal driver"

[[needs]]
on = "//dcl/interpreter"
why = "Commands go into DCL's buffer as it reads them"
+++

On VMS, DCL keeps the commands typed at its prompt in a recall buffer
of up to 254 commands in 4 KB. The up arrow or CTRL/B brings back the
one before, again for the one before that, and the down arrow goes
forward; the line can be edited before it is sent. `RECALL n` or
`RECALL string` puts a numbered command, or the last one starting with
the string, on the line; `/ALL` lists them, `/SEARCH` lists those with
a string, `/ERASE` empties the buffer, `/OUTPUT` and `/INPUT` save it to
a file and load it. DCL hands its buffer's line to the terminal driver
with an item list read, so a program's own input is kept apart.

vaxpunk's recall is the terminal driver's: each terminal's UCB keeps the
last 16 lines read with echo, `UCB$A_TT_RECALL`, and the arrows recall
them in any read with line editing
([ADR-0027](../../docs/adr/0027-terminals-are-ucbs-and-telnet-is-in-the-driver.md)).
So lines typed to a program show up among DCL's commands. DCL's own
buffer, and the `RECALL` command, are PRD-0003 tier 3 #29.
