+++
title = "PCSI (PRODUCT INSTALL)"
summary = "POLYCENTER Software Installation: install, remove and list products from kits"
aka = ["PCSI", "PRODUCT", "PRODUCT INSTALL", "PRODUCT SHOW PRODUCT", "PRODUCT REMOVE", "PRODUCT PACKAGE", ".PCSI", ".PCSI$COMPRESSED", "product database", "VMSINSTAL", "VMSINSTAL.COM", "KITINSTAL.COM"]
kind = "core"
intent = "aspiration"
research = "deferred"
refs = ["https://wasd.vsm.com.au/HELP/PRODUCT"]

[[feature]]
id = "vmsinstal"
title = "VMSINSTAL: a kit's save sets and its KITINSTAL.COM"
status = "todo"
needs = ["//utils/backup"]

[[feature]]
id = "install"
title = "PRODUCT INSTALL and REMOVE from sequential, compressed or reference kits"
status = "todo"

[[feature]]
id = "database"
title = "The product database: PRODUCT SHOW PRODUCT, SHOW HISTORY, SHOW OBJECT"
status = "todo"
needs = [":install"]

[[feature]]
id = "package"
title = "PRODUCT PACKAGE: a kit from a product description file"
status = "todo"
needs = [":install"]

[[feature]]
id = "patches"
title = "Patches, UNDO PATCH, and product dependencies"
status = "todo"
needs = [":database"]

[[needs]]
on = "//rms/core"
why = "Reads kits and writes the product database"

[[needs]]
on = "//dcl/procedures"
why = "Kits run DCL procedures at install time"

[[needs]]
on = "//utils/backup"
why = "VMSINSTAL kits are BACKUP save sets"
soft = true
+++

Software on VMS arrives as kits. The old way, still used for some
products, is `@SYS$UPDATE:VMSINSTAL`, which restores a kit's BACKUP
save sets and runs the kit's own `KITINSTAL.COM`. The POLYCENTER
Software Installation utility, `PRODUCT`, replaced it from OpenVMS
V6: a kit carries a product description file that says what files go
where, what other products it needs and what to ask; `PRODUCT INSTALL`
puts it in place and records every file in a product database, so
`PRODUCT SHOW PRODUCT`, `SHOW HISTORY` and `REMOVE` work. VSI ships
OpenVMS itself, its patches and layered products as PCSI kits.

vaxpunk has no installer: everything is on the system disk that
`vms/build.rs` writes on the host. A product database matters once
software is built or brought in separately from the system, which a
self-hosted vaxpunk, or ported layered products, would need. Nothing is
planned.
