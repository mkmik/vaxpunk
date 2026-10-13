+++
title = "Global sections"
summary = "Memory shared between processes, mapped from a file or the page file: $CRMPSC"
aka = ["$CRMPSC", "$MGBLSC", "$DGBLSC", "$UPDSEC", "$SECDEF", "GSD", "$CREATE_GDZRO", "$CRMPSC_GPFILE_64", "$MGBLSC_64", "private section", "global page file section", "PFN-mapped section", "memory-resident section", "shared page tables", "GBLSECTIONS", "GBLPAGES", "SYSGBL", "PRMGBL"]
kind = "core"
intent = "goal"
research = "outline"
refs = ["vms/exec/memory.mar", "vms/sysexe/gbltest.mar", "docs/prd/0003-multi-user-vms.md", "docs/adr/0028-shareable-images.md", "docs/design/0001-pal-interface.md", "https://wasd.vsm.com.au/htbin/helpgate/HELP/SYSTEM_SERVICES/_CRMPSC/ARGUMENTS"]

[[feature]]
id = "frame-caps"
title = "A PFN mapped at more than one address: a frame capability per extra mapping"
status = "done"
needs = ["/platform/pal"]
note = "main.c pfn_map and xmap; DESIGN-0001 Memory"

[[feature]]
id = "file-sections"
title = "$CRMPSC of a file: its blocks mapped, private or global"
status = "todo"
needs = ["#page-file-sections", "/files/ods2#qio-interface"]
note = "PRD-0003 tier 3 #20: SS$_IVCHNLSEC now, since no channel is to a file (t8y:/rms/core#ufo); image sections and installed images build on it"

[[feature]]
id = "delete"
title = "$DGBLSC: a section goes once no process maps it"
status = "done"
needs = ["#page-file-sections"]

[[feature]]
id = "update"
title = "$UPDSEC, and a writable file section's modified pages written back to the file"
status = "todo"
needs = ["#file-sections"]

[[feature]]
id = "permanent"
title = "Permanent and system-wide sections, with PRMGBL and SYSGBL"
status = "done"
needs = ["#page-file-sections", "/exec/privileges"]

[[feature]]
id = "logical-names"
title = "Section names translated through GBL$ logical names"
status = "todo"
needs = ["#page-file-sections"]

[[feature]]
id = "page-file-sections"
title = "Global page file sections: shared memory with no file behind it"
status = "done"
needs = ["#frame-caps"]
note = "On VMS backed by the page file (t8y:/exec/paging); here its pages are made with it and kept. Group or system, with an ident, a protection mask; GBLTEST"

[[feature]]
id = "pfn-sections"
title = "PFN-mapped sections: physical pages, such as a device's memory, mapped by number"
status = "todo"
needs = ["#frame-caps"]

[[feature]]
id = "memory-resident"
title = "Memory-resident sections and shared page tables, from reserved memory (Alpha V7.1)"
status = "todo"
needs = ["#page-file-sections"]

[[needs]]
on = "/exec/memory"
why = "Maps the section's pages into each process"

[[needs]]
on = "/platform/pal"
why = "The PAL holds the frame capabilities that map a page twice"

[[needs]]
on = "/exec/pool"
why = "Global section descriptors and the global page table"

[[needs]]
on = "/exec/logical-names"
why = "Section names are translated, and qualified by UIC group"

[[needs]]
on = "/exec/uic-protection"
why = "A section has an owner and a protection mask"

[[needs]]
on = "/exec/paging"
why = "Pages read as they are touched, and page file sections backed by it"
soft = true

[[needs]]
on = "/rms/core#ufo"
why = "$CRMPSC takes a channel to the file from a UFO open; soft only because RMS needs this component through the image activator"
soft = true
+++

A VMS section is a range of virtual pages backed by something: blocks of
a file, the page file, or physical pages. `$CRMPSC` creates and maps one;
a private section belongs to one process, a global section has a name
and any process may map it with `$MGBLSC`, so processes share its pages.
Group global sections are named within a UIC group, system ones across
the system (`SYSGBL`); permanent ones outlive their last user (`PRMGBL`).
Installed shared images are global sections, and so are RMS global
buffers and many applications' shared memory. A global page file
section is shared memory with no file of its own. Alpha V7.0 added
`_64` services that map sections in P2 space, and V7.1
memory-resident sections with shared page tables, for very large shared
memories such as database caches. SYSGEN's `GBLSECTIONS` and `GBLPAGES`
size the tables.

vaxpunk has global page file sections: `$CRMPSC` makes one, `$MGBLSC`
maps it, `$DGBLSC` deletes it, in `vms/exec/memory.mar`, with VMS's
flags, idents and protection, group and system, temporary and permanent.
Its pages are made with it, zeroed, not as they are touched. The PAL
maps a PFN at any number of addresses, with a frame capability for each
extra mapping ([DESIGN-0001](../../design/0001-pal-interface.md)).
Sections of a file wait for a channel to one: RMS has no `FAB$V_UFO`.
PRD-0003's backlog wants them (tier 3 #20) to share `LIBRTL.EXE`'s
pages between processes with `INSTALL /SHARED`
([ADR-0028](../../adr/0028-shareable-images.md) follow-ups), and to
page image sections in (tier 3 #21).
