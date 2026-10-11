+++
title = "apidoc"
summary = "The API reference page, generated from the sources, published on GitHub Pages"
aka = ["apidoc", "cargo run -p apidoc", "docs/api/index.html", "API reference"]
kind = "tool"
intent = "goal"
research = "done"
refs = ["apidoc/main.rs", "AGENTS.md", "docs/README.md", "docs/adr/0031-entropy-from-virtio-rng.md", ".github/workflows/apidoc.yml"]

[[feature]]
id = "page"
title = "Services, executive routines and data, PAL calls, macro libraries, condition values"
status = "done"
note = "From comment blocks, linkages, syssrv.mar, the $xxxDEF macros and DESIGN-0001's tables"

[[feature]]
id = "publish"
title = "Run on every PR; main's published to GitHub Pages"
status = "done"

[[feature]]
id = "not-vms"
title = "Departures from VMS tagged Not VMS"
status = "done"
note = "ADR-0031 decision 5"

[[feature]]
id = "strings"
title = "A ';' inside a string literal doesn't cut the line"
status = "todo"
note = "PRD-0003 tier 4 #45; main.rs ponytail"

+++

VMS documents its internals in the *OpenVMS AXP Internals and Data
Structures* book and the system service and RTL reference manuals.
vaxpunk's equivalent is one generated page: `cargo run -p apidoc` reads
the executive's and libraries' MACRO-32 and BLISS-64, the system service
vector, the macro libraries and DESIGN-0001's PAL tables, and writes
`docs/api/index.html`. Nothing on it is written by hand, so keeping it
right means keeping the sources' comments right. CI publishes main's,
alongside the browser demo.
