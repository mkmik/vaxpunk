+++
title = "QIO"
summary = "Partial, with no features; in a cycle with timers"
kind = "core"
intent = "induced"
research = "done"
status = "partial"
milestone = "m0"

[[needs]]
on = "/exec/timers"
why = "I/O timeouts"

[[needs]]
on = "/platform/sel4"
why = "Runs on it"

[[needs]]
on = "/lib/gfx"
soft = true
why = "Some day"
+++
