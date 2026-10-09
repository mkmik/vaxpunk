# vms: the OS

The VMS that runs on the PAL, written as if there were no seL4 under it.
The only way it reaches the machine is through VAX privileged
instructions, which vmacro compiles into PAL calls
([ADR-0002](../docs/adr/0002-root-task-is-the-pal.md)).

| Directory | What | On the system disk |
| --- | --- | --- |
| `exec/` | The executive in MACRO-32 and BLISS-64: scheduler, memory, system services, $QIO and drivers, Files-11, RMS, logical names. Linked as one image, its modules in file-name order | `[SYSEXE]EXEC.EXE` |
| `sysexe/` | The programs: DCL, the utilities, and the tests STARTUP runs. `lib/` is linked into every image; `dcl/`, `help/` and `rms/` only into the programs that use them | `[SYSEXE]*.EXE` |
| `sysexe/tpu/` | TPU, the Text Processing Utility ([PRD-0006](../docs/prd/0006-tpu-and-eve.md)): BLISS-64, and `fio.mar`, its I/O module; its tests run under vrun | `[SYSEXE]TPU.EXE` |
| [`crosstools/vtools/examples/c/cdemo/`](../crosstools/vtools/examples/c) | CDEMO, a BLISS-64 program that calls a C library through the library's BLISS-64 adapter; the C compiled by the cross gcc and converted by velf | `[SYSEXE]CDEMO.EXE` |
| `sysexe/librtl/` | The run-time library's `LIB$` routines, a shareable image every program but DCL calls ([ADR-0028](../docs/adr/0028-shareable-images.md)); its symbol vector is `librtl.opt` | `[SYSLIB]LIBRTL.EXE` |
| `cld/` | DCL's verbs, as CLD that vcdu compiles into DCL$TABLES ([ADR-0017](../docs/adr/0017-command-tables-from-cld-with-vcdu.md)) | in `DCL.EXE` and `HELP.EXE` |
| `sysmgr/` | SYSTEM's text files: SYSTARTUP_VMS.COM, SYLOGIN.COM, WELCOME.TXT | `[SYSMGR]` |
| `sysuaf.fdl`, `uafhash.rs` | SYSUAF.DAT's layout, and the password hash `build.rs` writes SYSTEM's with ([ADR-0029](../docs/adr/0029-loginout-and-sysuaf.md)) | `[SYSEXE]SYSUAF.DAT` |

`build.rs` compiles and links all of it with the vtools crates and writes
`sysdisk.img` with ods-image: `cargo build -p vms`. Only CDEMO's C needs
a C compiler, the cross gcc (`aarch64-elf-gcc`, `aarch64-linux-gnu-gcc`
or `CROSS_COMPILE`'s). Its file comment says which sources go into which
image.

The macro libraries (`lib.mlb`, `starlet.mlb`) are in
[crosstools/vtools/lib](../crosstools/vtools/lib). [DESIGN-0002](../docs/design/0002-executive-processes.md)
describes how the executive works, and [boot.md](../docs/boot.md) walks
through what it does at boot.
