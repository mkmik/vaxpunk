+++
title = "DCL"
band = "user"
summary = "The command language"
+++

DCL, the DIGITAL Command Language, is the command interpreter of every
interactive and batch process on VMS. It runs in supervisor mode in P1,
parses each command against command tables compiled from CLD, runs images
in the process, and runs command procedures: symbols, expressions,
lexical functions, `IF`, `GOTO`, `ON` and file I/O. Around it are
`SET COMMAND`, `HELP`, line recall and key definitions, the login
procedures, and `SPAWN` and `PIPE`, which run commands in subprocesses.

vaxpunk's DCL is `DCL.EXE`: `vms/sysexe/dcl.mar` and its modules in
`vms/sysexe/dcl/`, linked in P1
([ADR-0006](../../docs/adr/0006-cli-in-p1-runs-images-in-its-process.md)),
with every verb in CLD under `vms/cld/`
([ADR-0017](../../docs/adr/0017-command-tables-from-cld-with-vcdu.md),
[ADR-0018](../../docs/adr/0018-set-command-and-foreign-commands.md)).
PRD-0003 step 8 added block `IF`, `ON`, the lexical functions and data
lines. There is no `SPAWN` yet, so nothing runs in a subprocess.
