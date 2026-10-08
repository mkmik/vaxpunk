# pal: everything that knows seL4

Everything below the executive. Nothing above this directory includes
seL4 headers or makes seL4 calls.

| Directory | What | Output |
| --- | --- | --- |
| `src/` | The root task, which is the PAL ([ADR-0002](../docs/adr/0002-root-task-is-the-pal.md)): it loads `EXEC.EXE` from the system disk, runs the executive's processes as seL4 threads, answers their PAL calls and faults, drives the virtio disks and delivers the clock tick. Freestanding C | `roottask.elf` |
| `kernel/` | seL4 16.0.0 (submodule), built with its own CMake for the QEMU machine in `qemu.env` | `kernel.elf`, libsel4 |
| `shim/` | The Limine-protocol program that loads seL4 and the root task; see [shim/README.md](shim/README.md) | `shim.elf` |
| `tcpip/` | The TCP/IP component: lwIP (submodule) and a virtio-net driver, a seL4 thread the PAL starts. The executive only talks to it through the port, `tcpip/include/port.h` ([DESIGN-0003](../docs/design/0003-tcpip-port.md)) | `tcpip.elf`, embedded in `roottask.elf` |

Each one is a crate whose `build.rs` runs the C build into Cargo's
`OUT_DIR`. They need the cross toolchain from `scripts/setup-host.sh`.
[DESIGN-0001](../docs/design/0001-pal-interface.md) specifies the PAL
calls.
