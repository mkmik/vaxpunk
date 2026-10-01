# AGENTS.md

## Goal

vaxpunk is an OpenVMS clone for arm64. Detailed scope comes from a PRD (pending).
The toolchain sub-project (assembler, linker, runner) has its own PRD: [PRD-0001](docs/prd/0001-vtools.md).

Inspiration: [FreeVMS](https://github.com/rroart/freevms), a free VMS clone for x86_64.
FreeVMS is GPL-2.0 and vaxpunk is MIT: borrow ideas and designs, never copy code.

## Naming

- **vaxpunk**: the project and the OS as a whole.
- **vaxxine**: reserved for a component, tentatively a compat layer between the VMS kernel and the system. Exact role TBD.

## Docs

ADRs, PRDs and design documents live in [docs/](docs/), numbered per kind. Code follows the accepted ADRs.
To add or change one, follow [docs/README.md](docs/README.md).

## API reference

[docs/api/index.html](docs/api/index.html) documents every internal API: system services, executive
routines and data, PAL calls, macro libraries and condition values. `scripts/apidoc.py` (`just apidoc`)
generates it from the sources, so maintaining it means maintaining what it reads:

- Every global routine, `NAME::` or `.ENTRY`, has a comment block right above it whose first line is
  `NAME: what it does`, or `$NAME args: what it does` for a system service. Say what it takes and
  returns, then `IPL$_x.` if it must run at that IPL and `Uses Rn.` or `Keeps every register.`
- Every global data cell has a comment on its line or above it; every `.MACRO` a comment block above it;
  every `$xxxDEF` symbol a `; meaning` where the name doesn't say it.
- A new system service goes in syssrv.mar's vector, a new PAL call in DESIGN-0001's tables, a new
  module or macro library where the script globs for them.

Regenerate the page and commit it in the same PR as the change; CI fails when it is stale.
