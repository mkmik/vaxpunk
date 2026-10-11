+++
title = "License Management Facility"
summary = "LMF: product licenses (PAKs) registered, loaded at boot, checked by products"
aka = ["LMF", "LICENSE", "PAK", "Product Authorization Key", "LMF$LICENSE.LDB", "LICENSE REGISTER", "LICENSE LOAD", "VMSLICENSE.COM", "$GRANT_LICENSE", "SYS$LICENSE"]
kind = "core"
intent = "aspiration"
research = "deferred"
refs = ["https://typewritten.org/Manual/DEC/VMS/5.4/helplib.hlb/LICENSE.html"]

[[feature]]
id = "database"
title = "LICENSE REGISTER, LIST, ENABLE, DISABLE in SYS$COMMON:[SYSEXE]LMF$LICENSE.LDB"
status = "todo"
needs = ["//rms/indexed"]

[[feature]]
id = "load"
title = "LICENSE LOAD and UNLOAD; licenses loaded at boot"
status = "todo"
needs = [":database", "//utils/startup-shutdown"]

[[feature]]
id = "grant"
title = "$GRANT_LICENSE: a product asks for units when it starts"
status = "todo"
needs = [":load"]

[[needs]]
on = "//rms/indexed"
why = "The license database is an indexed file"

[[needs]]
on = "//exec/system-services"
why = "$GRANT_LICENSE and $LOOKUP_LICENSE are system services"
+++

From VMS V5, DEC's software checks for a license before it runs. A
Product Authorization Key (PAK) names the product, producer, version,
units, dates and a checksum; `LICENSE REGISTER` (or
`@SYS$UPDATE:VMSLICENSE`) puts it in the license database,
`LMF$LICENSE.LDB`, and `LICENSE LOAD`, done at every boot, makes it
active. Products call `$GRANT_LICENSE` to take units; the base
OS license and per-user licenses limit how many can log in. Hobbyist
and VSI community licenses are PAKs too.

vaxpunk is MIT and has no licenses to check, so it has no reason to
build LMF beyond fidelity: an empty `LICENSE LIST` and a `LICENSE`
command that accepts a PAK would make scripts and layered products'
installation procedures that expect it run. Nothing is planned.
