+++
title = "Host tools"
band = "host"
summary = "The cross toolchain and tools that build, test and show vaxpunk on the Mac or Linux"
+++

What runs on the development machine, macOS or Linux, to make what
vaxpunk runs. No VMS has these, since VMS builds itself. They are plain
Rust in `crosstools/` and the repository root, with no LLVM
([PRD-0001](../../prd/0001-vtools.md)). `vtools` is the toolchain:
`vmacro` (MACRO-32), `vbliss` (BLISS-64, stage 0), `vasm` (ARM64), `vcdu`
(CLD), `vlink`, `vlib`, `vdump` and `vrun`, around `vms-obj`, which
defines the object, library and image formats as OpenVMS Alpha's
transliterated to ARM64. `velf` and a stock cross gcc carry the C that the
C run-time library and SSL3 need. `ods` reads and writes Files-11 disk
images. `vms/build.rs` uses them all to build the system disk, the `boot`
crate boots it in QEMU and tests it, and OpenVMS Alpha in AXPbox is the
oracle the tools are checked against. `apidoc`, `todography` and the browser
demo show the project to people.

The long-term goal is a native, self-hosted toolchain: BLISS-64 compiling
itself on vaxpunk (`t8y:/lang/bliss`), MACRO-32 rewritten in BLISS
(`t8y:/lang/macro32`), and `LINK` and `LIBRARY` there too
(`t8y:/utils/linker`, `t8y:/utils/librarian`). Until then these tools make
every byte of vaxpunk.
