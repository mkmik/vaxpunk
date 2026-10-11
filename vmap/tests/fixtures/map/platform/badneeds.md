+++
title = "Bad needs"
summary = "Needs with both, neither, itself, and a bad label"
kind = "core"
intent = "aspiration"
research = "done"

[[feature]]
id = "Bad_Id"
title = "Bad id"
status = "todo"
needs = [":Bad_Id"]

[[needs]]
on = "//platform/sel4"
fuzzy = "and something else"

[[needs]]
why = "Nothing at all"

[[needs]]
on = "//platform/badneeds"

[[needs]]
on = "platform/sel4"
+++
