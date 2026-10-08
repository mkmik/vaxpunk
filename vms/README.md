# vms: the OS

The VMS that runs on the PAL, written as if there were no seL4 under it.
The only way it reaches the machine is through VAX privileged
instructions, which vmacro compiles into PAL calls
([ADR-0002](../docs/adr/0002-root-task-is-the-pal.md)).

| Directory | What | On the system disk |
| --- | --- | --- |
| `exec/` | The executive in MACRO-32 and BLISS-64: scheduler, memory, system services, $QIO and drivers, Files-11, RMS, logical names. Linked as one image, its modules in file-name order | `[SYSEXE]EXEC.EXE` |
| `sysexe/` | The programs: DCL, the utilities, and the tests STARTUP runs. `lib/` is linked into every image; `dcl/`, `help/` and `rms/` only into the programs that use them | `[SYSEXE]*.EXE` |
| `cld/` | DCL's verbs, as CLD that vcdu compiles into DCL$TABLES ([ADR-0017](../docs/adr/0017-command-tables-from-cld-with-vcdu.md)) | in `DCL.EXE` and `HELP.EXE` |
| `sysmgr/` | SYSTEM's text files: SYSTARTUP_VMS.COM, SYLOGIN.COM, WELCOME.TXT | `[SYSMGR]` |

`build.rs` compiles and links all of it with the vtools crates and writes
`sysdisk.img` with ods-image. It needs no C toolchain:
`cargo build -p vms`. Its file comment says which sources go into which
image.

The macro libraries (`lib.mlb`, `starlet.mlb`) are in
[crosstools/vtools/lib](../crosstools/vtools/lib). [DESIGN-0002](../docs/design/0002-executive-processes.md)
describes how the executive works, and [boot.md](../docs/boot.md) walks
through what it does at boot.
