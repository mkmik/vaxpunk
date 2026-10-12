+++
title = "INSTALL"
summary = "Makes images known: shared, privileged, protected or kept open"
aka = ["INSTALL", "INSTALL.EXE", "known image", "known file list", "INSTALL ADD", "INSTALL LIST", "/SHARED", "/PRIVILEGED", "/OPEN", "/HEADER_RESIDENT", "/RESIDENT", "/PROTECTED", "KFE"]
kind = "core"
intent = "aspiration"
research = "outline"
refs = ["docs/prd/0003-multi-user-vms.md", "docs/adr/0028-shareable-images.md", "docs/adr/0029-loginout-and-sysuaf.md", "vms/exec/process.mar", "https://docs.vmssoftware.com/vsi-openvms-system-management-utilities-reference-manual-volume-i-a-l/"]

[[feature]]
id = "open"
title = "INSTALL ADD /OPEN /HEADER_RESIDENT, LIST, REMOVE, REPLACE"
status = "todo"

[[feature]]
id = "privileged"
title = "/PRIVILEGED and /AUTHPRIV, in place of process.mar's KNOWN list"
status = "todo"
note = "PRD-0003 tier 1 #2 and ADR-0029: SETPWD gets SYSPRV from a fixed list until INSTALL"

[[feature]]
id = "shared"
title = "/SHARED: one copy of an image's pages, LIBRTL.EXE first"
status = "todo"
needs = ["/exec/global-sections"]
note = "PRD-0003 tier 3 #20, ADR-0028"

[[feature]]
id = "protected"
title = "/PROTECTED: user-written system services in a privileged shareable image"
status = "todo"
needs = ["/exec/shareable-images"]

[[feature]]
id = "writeable"
title = "/WRITEABLE: shared data sections"
status = "todo"
needs = ["#shared"]

[[feature]]
id = "resident"
title = "/RESIDENT: code in a granularity hint region, mapped with large pages"
status = "todo"
needs = ["#shared", "/exec/memory"]

[[needs]]
on = "/exec/installed-images#known-file-list"
why = "The executive's known file lists, which INSTALL adds to"

[[needs]]
on = "/exec/image-activator"
why = "Activates a known image with its privileges and shared sections"

[[needs]]
on = "/exec/privileges"
why = "Takes CMKRNL; /PRIVILEGED images enable privileges while they run"

[[needs]]
on = "/rtl/cli"
why = "Its own commands and qualifiers"
+++

`INSTALL` puts an image on a known file list, which the image activator
consults before it reads an image from disk. Installed `/OPEN`, its
file stays open; `/HEADER_RESIDENT`, its header stays in memory;
`/SHARED`, every process maps the same physical pages of its code and
read-only data, which is how `LIBRTL.EXE`, `DECC$SHR.EXE` and the other
system libraries are shared; `/PRIVILEGED`, it runs with privileges the
user doesn't have, which is how `LOGINOUT`, `SET PASSWORD` and `MAIL`
do their work; `/PROTECTED` with `/SHARED`, its routines are
user-written system services that change mode. On Alpha, `/RESIDENT`
puts code in large pages. Startup procedures run `INSTALL` to set all
this up before users log in.

vaxpunk has no `INSTALL`: installed images are a non-goal of PRD-0003.
Shareable images are mapped a copy per process
([ADR-0028](../../adr/0028-shareable-images.md)), and the
executive gives `SETPWD.EXE` `SYSPRV` from a fixed list in
`process.mar`, `KNOWN`, read by a name no logical name can redirect
([ADR-0029](../../adr/0029-loginout-and-sysuaf.md)). PRD-0003's
tier 3 #20 ends in `INSTALL /SHARED`, after frame capabilities and
global sections.
