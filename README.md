# vaxpunk

OpenVMS remake on arm64.

Milestone 0 boots an unmodified seL4 kernel on QEMU aarch64 through UEFI and
Limine, and runs a C root task that prints over the serial console:

```
EDK2 -> Limine (BOOTAA64.EFI) -> shim -> seL4 (kernel.elf) -> root task (roottask.elf)
```

## Host setup (once)

macOS (Apple Silicon, Homebrew) or Debian/Ubuntu (x86_64 or arm64):

```sh
scripts/setup-host.sh
```

It installs an aarch64 bare-metal C compiler, `cmake`, `ninja`, `dtc`, `uv`,
`mtools`, QEMU with its EDK2 firmware, and checks out the seL4 submodule.

## Build and run

```sh
make run
```

The first run downloads Limine, builds seL4 (about 10 seconds) and boots QEMU.
Quit QEMU with `Ctrl-A x`. EDK2 and Limine clear the console and move the
cursor around, so `scripts/serial-filter.py` turns their output into plain
lines before it reaches your terminal. From the shim's banner on, output
passes through untouched: terminal handling there is the guest's business.
Input is never filtered, and every key except `Ctrl-A` reaches the guest.
QEMU also saves the unfiltered console
in `out/serial.log`; read it with `less`, not `cat`. The serial console shows
Limine, the shim's placement banner, seL4's boot messages, then the root task:

```
vaxpunk shim: shim at 0x7fa68000, UART at 0x9000000
shim: kernel    0x40000000-0x40239000 entry 0xffffff8040000000
shim: DTB       0x40239000-0x4033a000
shim: root task 0x4033a000-0x40340000 vaddr 0x400000 entry 0x400000
shim: entering seL4
Bootstrapping kernel
...
Booting all finished, dropped to user space
hello from the root task
boot info: node 0 of 1, 53 untyped caps
  untyped 0: paddr 0x0 size 2^27 device
...
root task done
```

EDK2 prints a few `Error: Image at ... start failed` and `Tpm2...` lines
before Limine starts. That is normal for the firmware QEMU ships.

Nothing in the guest reads the serial input yet. After a few dozen keystrokes
QEMU stops reading the keyboard, and then `Ctrl-A x` no longer arrives. Stop
QEMU with `pkill -f qemu-system-aarch64` instead.

Day to day: edit `roottask/src/`, then `make run`. Only the root task is
rebuilt and the ESP image re-stitched. On an M3, the root task prints about
one second after the command.

## Layout

| Directory | What | Output |
| --- | --- | --- |
| `kernel/` | seL4 16.0.0 (submodule), built standalone with its own CMake | `kernel/out/`: `kernel.elf`, `include/` (libsel4), `kernel.dtb`, `platform_gen.json` |
| `shim/` | Limine-protocol program that loads seL4 and the root task; see [shim/README.md](shim/README.md) | `shim/out/shim.elf` |
| `roottask/` | the root task, freestanding C | `roottask/out/roottask.elf` |
| `image/` | Limine config and the ESP builder (mtools) | `out/esp.img` |
| `scripts/` | host setup, Limine download, QEMU wrapper and console filter | `out/serial.log` |
| `vtools/` | VMS-style toolchain in Rust: object and image formats, `vdump` to inspect them, and `vrun`, which runs images in QEMU; see [vtools/PRD.md](vtools/PRD.md) | `vtools/target/` |

Each component builds on its own with `make -C <dir>`. The components share
nothing but those output files. `shim/` and `roottask/` read `kernel/out/`,
so build the kernel first. The top-level `Makefile` only calls the others.

- The kernel rebuilds only when `kernel/config.cmake`, `kernel/qemu.env` or
  the seL4 commit change.
- `kernel/qemu.env` holds the QEMU CPU, RAM and GIC version. seL4 compiles in
  that machine's memory map, and `scripts/run-qemu.sh` reads the same file.
- Any root task can replace `roottask.elf`: it must be a static AArch64 ELF
  whose first segment is page aligned. seL4 calls its entry point with the
  boot info pointer in `x0`.
- Pins: seL4 by submodule commit (tag 16.0.0), Limine 11.4.1 by version and
  SHA-256 in `scripts/fetch-limine.sh`. The EDK2 firmware comes from the
  QEMU install (`EDK2_FW=` overrides it).
- `vtools/` is a Cargo workspace instead: `cd vtools && cargo test`. It needs
  a Rust toolchain besides what `setup-host.sh` installs. Its tests run images
  under `vrun` in QEMU; `VRUN_FLAGS=--hvf cargo test` runs them under HVF.

## Toolchain

Each component picks its compiler with `CROSS_COMPILE` (default:
`aarch64-elf-` if installed, else `aarch64-linux-gnu-`), for example
`make CROSS_COMPILE=aarch64-none-elf-`. `CC=` overrides the shim and root
task compiler alone. `vrun`'s boot stub is assembled with the same
`CROSS_COMPILE` binutils.

## Debugging

`scripts/run-qemu.sh --gdb` starts QEMU halted with a GDB server on port 1234.
In another terminal:

```sh
lldb roottask/out/roottask.elf -o 'gdb-remote 1234' -o 'b main' -o c
gdb-multiarch roottask/out/roottask.elf -ex 'target remote :1234' -ex 'b main' -ex c
```

`scripts/run-qemu.sh --hvf` runs under Hypervisor.framework instead of TCG.
It boots the same kernel, which is why `kernel/qemu.env` picks GICv3 (HVF
does not emulate GICv2). This is best effort: the kernel is built for a
Cortex-A57 while HVF offers only `-cpu host`, and seL4 warns that the
counter runs at 24 MHz instead of the 62.5 MHz it was built for.
