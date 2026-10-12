# todography

`todography` checks and queries the map in [docs/map/](../docs/map/): one Markdown file per
component, with TOML front matter. [docs/map/README.md](../docs/map/README.md) says
what goes in a file. This page says what the tool does with it.

```sh
cargo run -p todography -- lint                    # check the map; exit 1 on errors
cargo run -p todography -- lint --strict           # exit 1 on warnings too
cargo run -p todography -- show exec/mailboxes     # one component or feature
cargo run -p todography -- why rtl/crtl            # the chains from the goals down to it
cargo run -p todography -- rdeps --all exec/qio    # what needs it, and what needs those
cargo run -p todography -- next                    # wanted work whose hard needs are met
cargo run -p todography -- next --all --milestone m1   # and the blocked work, with its blockers
cargo run -p todography -- debt                    # wanted components not researched yet
cargo run -p todography -- json                    # everything, as JSON
cargo run -p todography -- build                   # write out/map/index.html
```

todography finds the map by looking, in the current directory and the ones
above it, for `MAP.toml` (inside the map) or `docs/map/MAP.toml` (at the
workspace root). `--map DIR` points it somewhere else. `build` writes under
the workspace root, the directory above `docs/`, unless `--out PATH` says
otherwise. A label on the command line may be `t8y:/exec/qio`, `/exec/qio`
or just `exec/qio`, with `#feature` after it.

## What it works out

- **Status.** A component with features takes its status from the ones
  not dropped: all done is `done`, some done is `partial`, any wip is
  `wip`, else `todo`. If every feature is dropped, so is the component.
  A component without features has its `status` field, or `todo`.
- **Met.** A need on a feature is met when the feature is done. A need on a
  component is met when the component is `partial` or `done`. A fuzzy need
  is never met.
- **Wanted.** Goals, and whatever a wanted component needs, through any
  number of steps. Soft needs and fuzzy needs don't count. Neither do the
  needs of a dropped feature.
- **Why paths.** For a wanted component that isn't a goal, up to three of
  the shortest chains of needs from a goal down to it.
- **Work.** Each feature that isn't done or dropped, and each component
  without features that isn't done or dropped. It is blocked by its own
  unmet hard needs and by its component's. It is ready when it is wanted
  and nothing blocks it.
- **Research debt.** A wanted component with `research = "deferred"` or a
  fuzzy need.

## Lint

Errors: a file that doesn't parse, an unknown field or value, a bad id or
label, a label that names nothing, a need with both or neither of `on` and
`fuzzy`, a duplicate feature id, `status` on a component with features, a
component that needs itself, an unknown milestone or band, a directory
without `AREA.md`, and a missing title, summary, kind or intent.

Warnings: a summary over 100 characters, a missing `research` (taken as
`deferred`), an induced component nothing needs, an aspiration that
something wanted needs, and a cycle of hard needs between components. A
feature that needs another feature of its own component is fine.

## The page

`build` puts the JSON from `todography json` into
[assets/map.html](assets/map.html) in place of the text `__MAP_DATA__`,
which sits in the page's script as `const MAP = __MAP_DATA__;`. It writes
the page even when lint finds errors; they are in the data, under `lint`.
The page reads exactly the fields `json` writes, so change
[src/json.rs](src/json.rs) and the page together. The template is built
into the binary with `include_str!`, so rebuild todography after editing it.

`generated` is the time of the run, or `SOURCE_DATE_EPOCH` if it is set.

The page is one file with no scripts, fonts or styles from elsewhere. It
works on a laptop and on a phone, in light and dark mode. The header has
the map's title, when it was made, the counts (components, wanted, done,
partial, work ready now), a search box (title, aka, label, summary; `/`
focuses it) that opens a component in Focus, and a badge with the lint
errors and warnings when there are any. Four tabs switch views. The view
lives in the URL after `#`, so back, forward and links work:
`#view=map&show=all`, `#focus=t8y:/exec/mailboxes`, `#focus=t8y:/rtl/crtl%23stdio`,
`#view=roadmap&milestone=m0&area=net`, `#view=choose`.

- **Map**: the bands top to bottom, each band's areas as boxes, each
  area's components as tiles that show only the title. The tile's left
  edge and fill give its status, with a thin bar for how much of a
  partial one is done. Goals are bold. Wanted tiles are solid; tiles that
  aren't wanted are dashed and grey. "Wanted only", "Wanted +
  aspirations" (the default) and "Everything" choose what shows. In the
  default, induced components nothing wanted needs are hidden, and an
  aspiration whose hard needs reach them shows `+N`. Hovering a tile (on
  a phone, the first tap) outlines what it needs and what needs it, draws
  lines to them, dims the rest and shows its summary at the bottom; a
  click (the second tap) opens Focus. Clicking an area's name shows its
  `AREA.md`.
- **Focus**: one component as a card in the middle: title, label, aka,
  summary, chips (kind, intent, wanted, status, research, milestone,
  research debt), progress, whether it is ready or what blocks it, its
  features as a checklist (status, milestone, note, refs, the feature's
  own needs, its blockers), the why chains from the goals down to it, its
  description, its refs (URLs as links, repository paths as text) and its
  fuzzy needs. On the left, what it needs: one node per component, with
  each need's `why`, the feature it is for and the feature it points at,
  met or not; soft needs dashed, needs on the map's missing labels and
  fuzzy needs in grey. On the right, what needs it. Curves join them to
  the card; on a narrow screen they stack without curves. Clicking a
  neighbour re-centres on it, and a trail shows where you've been.
  Work that is ready, here and in the roadmap, has a copy button: it puts
  a prompt for an agent on the clipboard, with the work's title, label,
  note or summary, its map file and the `show` command for the rest.
- **Roadmap**: the wanted work in columns. "Ready now" is the work
  nothing blocks; a later column holds work whose blockers are all in the
  columns before it. A blocker that is a feature, or a component without
  features, counts at its own work item's column; a component with
  features counts at its earliest unfinished feature, since one done
  feature makes it usable; anything else counts as ready. Cycles are cut
  where the walk meets itself. It filters by milestone (m1 by default
  when there is m1 work) and by area; columns count blockers outside the
  filter too, and blockers that are ready now are marked.
- **Choose**: the aspirations nothing wanted needs, by band and area,
  under a filter that stays at the top while the list scrolls (words match
  the title, label, summary, other names and area; "Previewed only" keeps
  the previewed ones), each with what choosing it would pull in: the components its hard needs
  reach that aren't wanted today (through features that aren't dropped),
  and how many features, fuzzy needs and unresearched components those
  hold. "Preview as chosen" treats an aspiration as a goal on this page
  only: wanted, ready, research debt and the why chains are worked out
  again in the browser, and the map, roadmap, focus and counts follow. The
  preview is kept in the browser's local storage when it can be; choosing
  for real means setting `intent = "goal"` in the file. Below is the
  research debt: wanted components not researched yet, with their fuzzy
  needs.

## Tests

`cargo test -p todography` runs todography on the small map in
[tests/fixtures/map](tests/fixtures/map). Its mistakes are on purpose:
each should come out as one lint line.
