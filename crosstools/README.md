# crosstools: host tools for vaxpunk

What runs on the development machine, macOS or Linux, to make what
vaxpunk runs and the disks it reads. Both are plain Rust and need no
cross C toolchain.

| Directory | What |
| --- | --- |
| [vtools/](vtools/README.md) | The toolchain: MACRO-32 and BLISS-64 compilers, an ARM64 assembler, a CLD compiler, a linker and a librarian, which write vaxpunk's object, library and image formats, and `vrun`, which runs images in QEMU |
| [ods/](ods/README.md) | Files-11 ODS-2 and ODS-5: a library, the `ods` CLI and a FUSE mount, which read and write disk images |

[vms/build.rs](../vms/build.rs) uses both to build the system disk: vtools
compiles and links the images, and ods-image writes them to the volume.
