+++
title = "Editor"
summary = "A goal that needs features of an induced library"
kind = "core"
intent = "goal"
research = "done"
refs = ["docs/editor.md"]

[[feature]]
id = "basic"
title = "Open, edit, save"
status = "done"

[[feature]]
id = "undo"
title = "Undo"
status = "todo"
milestone = "m0"

[[feature]]
id = "search"
title = "Search with regular expressions"
status = "todo"
milestone = "m1"
needs = ["/lib/regex#engine"]

[[feature]]
id = "spell"
title = "Spelling"
status = "todo"
needs = ["/lib/regex#unicode"]

[[feature]]
id = "macros"
title = "Keyboard macros"
status = "dropped"
needs = ["/lib/gfx"]

[[needs]]
on = "/exec/qio"
why = "Reads and writes files"

[[needs]]
fuzzy = "a terminal driver"
why = "Draws the screen"
+++

The editor. It has </script> in its body, which the page must survive.
