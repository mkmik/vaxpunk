+++
title = "Installed images"
summary = "The known file list: images kept open, shared or run with privileges"
aka = ["known image", "known file list", "KFE", "INSTALL", "/PRIVILEGED", "/SHARED", "/OPEN", "/HEADER_RESIDENT", "/PROTECTED", "/RESIDENT", "/WRITEABLE", "VMS$IMAGES_MASTER.DAT"]
kind = "core"
intent = "goal"
research = "outline"
refs = ["docs/prd/0003-multi-user-vms.md", "docs/adr/0028-shareable-images.md", "docs/adr/0029-loginout-and-sysuaf.md", "docs/design/0002-executive-processes.md", "vms/exec/process.mar", "https://h41379.www4.hpe.com/doc/84final/6048/6048pro_033.html"]

[[feature]]
id = "fixed-list"
title = "A fixed known image list in process.mar: SETPWD.EXE runs with SYSPRV"
status = "done"
milestone = "m1"
note = "PRD-0003 step 4, tier 1 #2: a fixed list until INSTALL"

[[feature]]
id = "known-file-list"
title = "The known file list in the executive, kept by INSTALL ADD, REMOVE and LIST"
status = "todo"
note = "A non-goal of PRD-0003; INSTALL is t8y:/utils/install"

[[feature]]
id = "privileged"
title = "/PRIVILEGED: an image's privileges while it runs, from the list"
status = "todo"
needs = ["#known-file-list", "/exec/privileges"]

[[feature]]
id = "open"
title = "/OPEN and /HEADER_RESIDENT: activation without a directory lookup or header read"
status = "todo"
needs = ["#known-file-list"]

[[feature]]
id = "shared"
title = "/SHARED: the image's read-only sections a global section every process maps"
status = "todo"
needs = ["#known-file-list", "/exec/global-sections#file-sections"]
note = "PRD-0003 tier 3 #20; ADR-0028 follow-ups: LIBRTL.EXE's pages shared"

[[feature]]
id = "protected"
title = "/PROTECTED: user-written system services, an image's code callable in inner modes"
status = "todo"
needs = ["#known-file-list"]

[[feature]]
id = "resident"
title = "/RESIDENT and /SHARED=ADDRESS_DATA: code kept in memory, addresses fixed at install"
status = "todo"
needs = ["#shared"]

[[needs]]
on = "/exec/image-activator"
why = "Looks images up in the list before the disk"

[[needs]]
on = "/exec/pool"
why = "Known file entries"

[[needs]]
on = "/exec/privileges"
why = "CMKRNL to install; image privileges"
+++

VMS keeps a known file list of installed images, which the `INSTALL`
utility maintains and `SYS$STARTUP` fills at boot. An installed image is
found without a directory lookup, and may be kept open (`/OPEN`), have
its header kept in memory (`/HEADER_RESIDENT`), have its read-only
sections shared by every process as global sections (`/SHARED`), give
its privileges to whoever runs it while it runs (`/PRIVILEGED`, how
`SET PASSWORD` and `MAIL` work for ordinary users), or hold
user-written system services (`/PROTECTED`). Alpha added resident code
and preassigned addresses for shareable images.

vaxpunk has a stand-in: `KNOWN` in `vms/exec/process.mar` lists images
that get privileges while they run, each read from a fixed file on the
system disk, and `SETPWD.EXE` gets `SYSPRV` to write `SYSUAF.DAT`
(PRD-0003 step 4). The list is fixed until `INSTALL`. `INSTALL` itself
and processes sharing an image's pages are non-goals of Milestone 1,
listed after it in PRD-0003's backlog (tier 3 #20).
