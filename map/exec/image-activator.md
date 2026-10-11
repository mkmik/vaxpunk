+++
title = "Image activator"
summary = "Maps an image's sections into the process and starts it: $IMGACT, RUN"
aka = ["IMG$ACTIVATE", "$IMGACT", "$IMGFIX", "image header", "EIHD", "EISD", "image section", "RUN", "LIB$INITIALIZE", "process.mar"]
kind = "core"
intent = "goal"
research = "done"
milestone = "m0"
refs = ["docs/design/0002-executive-processes.md", "docs/adr/0006-cli-in-p1-runs-images-in-its-process.md", "docs/adr/0028-shareable-images.md", "docs/prd/0001-vtools.md", "crosstools/vtools/docs/image-format.md", "vms/exec/process.mar", "vms/exec/f11.mar", "docs/prd/0003-multi-user-vms.md"]

[[feature]]
id = "basic"
title = "Reads the header into pool, maps each section at its link address, protects it"
status = "done"
milestone = "m0"

[[feature]]
id = "cli-images"
title = "Images linked in P1 are command interpreters, mapped for supervisor mode"
status = "done"
milestone = "m0"
refs = ["docs/adr/0006-cli-in-p1-runs-images-in-its-process.md"]

[[feature]]
id = "imgact"
title = "$IMGACT: a new image in the current process, with its command's parse"
status = "done"
milestone = "m0"

[[feature]]
id = "execute-access"
title = "Execute access to the image file checked"
status = "done"
milestone = "m1"
note = "PRD-0003 step 2"

[[feature]]
id = "demand-paging"
title = "Sections mapped from the file and read page by page as they are touched"
status = "todo"
needs = ["//exec/memory:page-faults", "//exec/global-sections"]
note = "PRD-0003 tier 3 #21: read whole through a 4 KB buffer in pool (FIL$READIMG); PRD-0007 M0 needs it"

[[feature]]
id = "relocation"
title = "Main images that move, not only shareable ones"
status = "todo"
note = "PRD-0003 tier 3 #20; process.mar ponytail: an image runs where it is linked"

[[feature]]
id = "activate-only"
title = "$IMGACT only activates, and the command interpreter calls the image"
status = "todo"
note = "PRD-0003 tier 3 #32; process.mar ponytail"

[[feature]]
id = "lib-initialize"
title = "LIB$INITIALIZE routines called before the transfer address"
status = "todo"

[[needs]]
on = "//exec/memory"
why = "Makes and protects the sections' pages in the current process's P0 or P1"

[[needs]]
on = "//exec/pool"
why = "The image header is read into pool"

[[needs]]
on = "//files/ods2"
why = "FIL$OPENFILE and FIL$READIMG read the image"

[[needs]]
on = "//exec/logical-names"
why = "SYS$SYSTEM: and the image name translated"

[[needs]]
on = "//host/vlink"
why = "The images are vlink's, in OpenVMS Alpha's format"
+++

The VMS image activator turns an executable file into a running
program. It reads the image header, which lists the image sections, maps
each into P0 (or P1 for a command interpreter): code and read-only data
shared or mapped from the file, writable data copied on reference,
demand-zero pages for the rest. It then activates the shareable images
the image was linked against and fixes up its references to them. The
header's transfer addresses put the debugger and `LIB$INITIALIZE`
routines before the program's own, and the command interpreter calls
them in turn. Installed images skip the file lookup and share their
sections. Pages come in as they are touched, through page faults.

vaxpunk's is `IMG$ACTIVATE` in `vms/exec/process.mar`. It reads the
header into pool with `FIL$OPENFILE`, checks it, and maps each section at
its link address, reading its contents into zeroed pages through a 4 KB
buffer in pool (`FIL$READIMG`), then sets each section's protection:
read and execute, read only, or writable, for user mode in P0 and
supervisor mode in P1 ([ADR-0006](../../docs/adr/0006-cli-in-p1-runs-images-in-its-process.md)).
`$IMGACT` runs an image under DCL, and also calls it, which VMS's
doesn't. The shareable images it calls are mapped after it
(`//exec/shareable-images`), and known images get their privileges here
(`//exec/installed-images`). Missing: demand paging (PRD-0003 tier 3
#21), which needs page faults and global sections, and main images that
move (tier 3 #20).
