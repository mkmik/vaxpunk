# The map

The map says what vaxpunk is made of, what a whole OpenVMS is made of, how
the pieces depend on each other, and how far along each one is. It is data:
one Markdown file per component, which coding sessions edit next to the
code they change. `todography` (in [todography/](../todography/)) checks it and draws it.

```sh
cargo run -p todography -- lint            # check every label; CI runs this
cargo run -p todography -- build           # write out/map/index.html, the interactive map
cargo run -p todography -- show //exec/mailboxes      # one component, its needs and who needs it
cargo run -p todography -- why //rtl/crtl  # the chain from a component up to the goals that pull it in
cargo run -p todography -- next            # the work whose dependencies are done
cargo run -p todography -- debt            # wanted components whose dependencies aren't researched yet
```

## Files

Each directory under `map/` is an **area**, such as `exec` or `net`. Its
`AREA.md` says what the area is and where it sits in the picture. Every
other `.md` file in it is a **component**, one thing you could point at:
a system service family, a utility, a driver, a layered product.

A component's **label** is its path without `.md`, as in Bazel:
`map/exec/mailboxes.md` is `//exec/mailboxes`. A **feature** of it is
`//exec/mailboxes:attention-asts`. In the component's own file, `:attention-asts`
is enough. A label that names nothing is an error, so a typo can't
quietly cut a link.

## A component

```markdown
+++
title = "Mailboxes"
summary = "Message queues between processes, as MBAn: devices"
aka = ["MBX", "$CREMBX"]
kind = "core"
intent = "goal"
research = "done"
refs = ["docs/adr/0019-mailboxes.md", "vms/exec/mbdriver.mar"]

[[feature]]
id = "basic"
title = "$CREMBX, $DELMBX, reads and writes"
status = "done"

[[feature]]
id = "attention-asts"
title = "Read and write attention ASTs"
status = "todo"
milestone = "m1"

[[needs]]
on = "//exec/qio"
why = "Its reads and writes are $QIO requests"
+++

What a mailbox is, in a paragraph or a few. The map shows this only when
you open the component.
```

The fields between the `+++` lines are TOML:

| Field | Meaning |
| --- | --- |
| `title` | The short name the map shows: `OPCOM`, `Mailboxes` |
| `summary` | One line: what it is |
| `aka` | Other names it goes by, for search |
| `kind` | What it is in a VMS: `core` (part of the base OS), `layered` (a layered product DEC or a partner sold), `glue` (something vaxpunk needs that isn't a VMS piece, such as the OpenSSL API on Mbed TLS), `platform` (below the executive: seL4, the PAL, boot), `tool` (host tools that build vaxpunk) |
| `intent` | Why it is on the map: `goal` (we want it), `aspiration` (we might want it some day; not chosen), `induced` (no one wants it for itself; it's here because something else needs it) |
| `status` | `done`, `partial`, `wip`, `todo` or `dropped`, for a component with no features. With features, the status comes from them |
| `research` | How well its own dependencies are known: `done`, `outline` (the big ones are known), `deferred` (not looked into; its needs are guesses) |
| `milestone` | The milestone it belongs to, from `map/MAP.toml` |
| `refs` | Repository paths or URLs that say more: PRDs, ADRs, sources, manuals |

`[[feature]]` entries list what the component does, in the order it
grows: the first is usually the basic thing that works, the later ones
the gaps. Each has an `id` (lowercase, digits and `-`), a `title`, a
`status` (`done`, `wip`, `todo` or `dropped`), and may have a
`milestone`, `refs` and a `note`.

`[[needs]]` entries list what the component depends on. Each has either
`on`, a label, or `fuzzy`, words for a dependency that isn't on the map
yet ("Motif's widget set"). `why` says what for. `soft = true` marks a
dependency it can live without for a while. A component's `[[needs]]` and
a feature's `needs` (a list of labels) both count.

## What the map works out for itself

- **Wanted**: a goal, and everything a wanted component needs, through
  any number of steps. An aspiration nothing wanted needs, and the
  induced pieces only it needs, are not wanted: the map dims them and
  hides their detail until you choose them.
- **Why**: an induced component exists for the wanted components that
  need it; `todography why` and the map show the chain up to the goals.
- **Ready** and **blocked**: unfinished work whose hard dependencies are
  met is ready; the rest is blocked, and the map says by what. A need on a
  feature is met when the feature is done. A need on a whole component is
  met once the component is usable, `partial` or `done`: point at the
  feature when you need more than its basics. Soft needs never block, and
  don't make what they point at wanted.
- **Research debt**: a wanted component with `research = "deferred"` or
  a `fuzzy` need. Choosing an aspiration turns its fuzzy needs into debt,
  which is the moment to look into them and replace them with labels.

## When you change code

- A feature lands: set its `status = "done"`.
- You find something missing in another layer: add a `[[feature]]` with
  `status = "todo"` to that layer's component, and a `needs` on it from
  yours, in the same change. Record it where it belongs, not where you
  were working.
- A new piece appears because something needed it: add a component with
  `intent = "induced"`, and the `needs` that pulled it in.
- `cargo run -p todography -- lint` before you commit.
