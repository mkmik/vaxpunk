# vtools: the toolchain

VMS-style cross tools in Rust that build vaxpunk's ARM64 images on the
host: compilers, an assembler, a linker and a librarian, which write
vaxpunk's own object, library and image formats. vms/build.rs uses them
to make the system disk. [PRD-0001](../docs/prd/0001-vtools.md) is the
plan.

| Crate | What |
| --- | --- |
| `crates/vmacro` | MACRO-32 compiler for ARM64, as AMACRO compiled MACRO-32 for Alpha ([docs/macro32.md](docs/macro32.md), [docs/amacro.md](docs/amacro.md)) |
| `crates/vbliss` | BLISS-64 compiler for ARM64 ([docs/vbliss.md](docs/vbliss.md), [docs/bliss64.md](docs/bliss64.md)) |
| `crates/vasm` | ARM64 assembler with VMS-style directives; the other compilers emit through it ([docs/assembler.md](docs/assembler.md)) |
| `crates/vcdu` | Compiles CLD into command tables, like SET COMMAND/OBJECT ([docs/command-tables.md](docs/command-tables.md)) |
| `crates/vlink` | Links object modules into an executable image ([docs/linker.md](docs/linker.md)) |
| `crates/vlib` | Puts object modules into object libraries, like LIBRARY/OBJECT |
| `crates/vdump` | Decoded dump of objects, libraries and images, like ANALYZE/OBJECT and ANALYZE/IMAGE |
| `crates/vms-obj` | The object, library and image formats ([docs/object-format.md](docs/object-format.md), [docs/library-format.md](docs/library-format.md), [docs/image-format.md](docs/image-format.md)); `no_std` with `alloc` |
| `crates/vrun` | Runs an image at EL0 in a bare-metal QEMU machine, for quick checks of compiler output without booting vaxpunk ([docs/runner-abi.md](docs/runner-abi.md)) |
| `crates/vdefs` | Generates BLISS require files (`lib/*.r64`, `*.req`) from the macro libraries: `just defs` |

The other directories:

- `lib/`: the macro libraries, `lib.mlb` for the executive and
  `starlet.mlb` for programs, as on VMS, plus `call.mlb`, `pic.mlb` and
  `vrun.mlb`, the BLISS require files generated from them, and modules
  shared by the executive and the programs.
- `bliss/`: vasm rewritten in BLISS-64, with `fio.mar`, its I/O under vrun.
  vrun's `vasm_port` test checks that it assembles `tests/vasm-port.txt`
  the same way the Rust vasm does.
- `examples/`: small MACRO-32, BLISS and vasm programs, each directory
  with its own README.
- `tests/`: sources with their expected listings and output, which the
  crates' tests compare against.

The crates are default members of the repository's Cargo workspace, so
`cargo build` and `cargo test` cover them. vrun's tests need
`qemu-system-aarch64` and cross binutils.
