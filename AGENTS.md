# AGENTS.md

## Goal

vaxpunk is an OpenVMS clone for arm64. Detailed scope comes from a PRD (pending).
The toolchain sub-project (assembler, linker, runner) has its own PRD: [vtools/PRD.md](vtools/PRD.md).

Inspiration: [FreeVMS](https://github.com/rroart/freevms), a free VMS clone for x86_64.
FreeVMS is GPL-2.0 and vaxpunk is MIT: borrow ideas and designs, never copy code.

## Naming

- **vaxpunk**: the project and the OS as a whole.
- **vaxxine**: reserved for a component, tentatively a compat layer between the VMS kernel and the system. Exact role TBD.
