# AGENTS.md

## Goal

vaxpunk is an OpenVMS clone for arm64. The current milestone, and the ranked backlog of what's left, is
[PRD-0003](docs/prd/0003-multi-user-vms.md).
The toolchain sub-project (assembler, linker, runner) has its own PRD: [PRD-0001](docs/prd/0001-vtools.md).

Inspiration: [FreeVMS](https://github.com/rroart/freevms), a free VMS clone for x86_64.
FreeVMS is GPL-2.0 and vaxpunk is MIT: borrow ideas and designs, never copy code.

## Naming

- **vaxpunk**: the project and the OS as a whole.
- **vaxxine**: reserved for a component, tentatively a compat layer between the VMS kernel and the system. Exact role TBD.

## Docs

ADRs, PRDs and design documents live in [docs/](docs/), numbered per kind. Code follows the accepted ADRs.
To add or change one, follow [docs/README.md](docs/README.md).

[docs/boot.md](docs/boot.md) walks through the boot sequence step by step. A PR that changes what boot
does (a new stage, a new init routine, a new STARTUP process) updates it too. Write it like the rest of
that file: plain words a newcomer can follow, the real names (routines, files, console lines), and no
silly analogies.

## API reference

The [API reference](https://mkmik.github.io/vaxpunk/docs/) documents every internal API: system services, executive
routines and data, PAL calls, macro libraries and condition values. `cargo run -p apidoc` ([apidoc/](apidoc/))
generates it from the sources into docs/api/index.html, so maintaining it means maintaining what it reads:

- Every global routine, `NAME::` or `.ENTRY`, has a comment block right above it whose first line is
  `NAME: what it does`, or `$NAME args: what it does` for a system service. Say what it takes and
  returns, then `IPL$_x.` if it must run at that IPL and `Uses Rn.` or `Keeps every register.`
- Every global data cell has a comment on its line or above it; every `.MACRO` a comment block above it;
  every `$xxxDEF` symbol a `; meaning` where the name doesn't say it.
- Where an interface isn't VAX or Alpha VMS's, a paragraph of its comment block that begins `vaxpunk:`
  says how it differs; apidoc tags the entry *Not VMS* ([ADR-0031](docs/adr/0031-entropy-from-virtio-rng.md)).
- A new system service goes in syssrv.mar's vector, a new PAL call in DESIGN-0001's tables, a new
  module or macro library where the script globs for them.

The page isn't committed: CI checks that it generates on every PR and publishes main's to GitHub Pages.
