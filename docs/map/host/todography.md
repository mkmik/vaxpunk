+++
title = "todography (this map)"
summary = "Checks the map's files, answers questions about them, and draws the map page"
aka = ["todography", "cargo run -p todography", "docs/map/", "MAP.toml", "t8y:", "out/map/index.html"]
kind = "tool"
intent = "goal"
research = "done"
refs = ["todography/README.md", "docs/map/README.md", "todography/src", "todography/assets/map.html"]

[[feature]]
id = "lint"
title = "lint: every label, field and value checked; errors and warnings"
status = "done"

[[feature]]
id = "queries"
title = "show, why, rdeps, next, debt, json"
status = "done"

[[feature]]
id = "page"
title = "build: one self-contained page with Map, Focus, Roadmap and Choose views"
status = "done"

[[feature]]
id = "content"
title = "Every component of vaxpunk and of a whole OpenVMS on the map"
status = "wip"
note = "docs/map/: the areas being written"

[[feature]]
id = "ci"
title = "CI runs todography lint on every PR"
status = "todo"
note = "docs/map/README.md says CI runs it; no workflow does yet"

+++

Not a VMS piece: vaxpunk's map of itself and of a whole OpenVMS. Each
component is a Markdown file under `docs/map/` with TOML front matter: what
it is, why it is on the map (goal, induced or aspiration), its features
and their status, and what it needs. `todography` checks every label, works
out what is wanted, ready and blocked, and writes one page with the map,
a focus view per component, a roadmap and a page for choosing
aspirations. Coding sessions edit the map next to the code they change.
