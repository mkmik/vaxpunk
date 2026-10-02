# vaxpunk

OpenVMS remake on arm64.

Milestone 0 boots an unmodified seL4 kernel on QEMU aarch64 through UEFI and
Limine, and runs a C root task that prints over the serial console. The root
task is the PAL ([ADR-0002](docs/adr/0002-root-task-is-the-pal.md)): it
starts `EXEC.EXE`, the executive, an image compiled from MACRO-32 with
vtools, as a task of its own and serves the PAL calls its privileged
instructions make ([DESIGN-0001](docs/design/0001-pal-interface.md)). The
executive manages memory, and creates, schedules and deletes processes,
each a thread running an image from the system disk, which take turns on
one CPU and synchronize with IPL and event flags
([ADR-0003](docs/adr/0003-one-cpu-many-threads.md),
[DESIGN-0002](docs/design/0002-executive-processes.md)). seL4 is
built with its MCS (mixed-criticality scheduling) API: threads run on
scheduling contexts with a budget and period, and IPC replies go through
reply objects.

```
EDK2 -> Limine (BOOTAA64.EFI) -> shim -> seL4 (kernel.elf) -> root task (roottask.elf) -> EXEC.EXE -> processes
```

## Host setup (once)

macOS (Apple Silicon, Homebrew) or Debian/Ubuntu (x86_64 or arm64):

```sh
scripts/setup-host.sh
```

It installs an aarch64 bare-metal C compiler, `cmake`, `ninja`, `dtc`, `uv`,
`mtools`, QEMU with its EDK2 firmware, `just`, and checks out the seL4 submodule.
Cargo drives the whole build, so you also need a Rust toolchain, for example
from [rustup](https://rustup.rs).

## Build and run

```sh
cargo run -p boot
```

The first run builds seL4 (about 10 seconds), downloads Limine and boots QEMU.
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
shim: kernel    0x40000000-0x40243000 entry 0xffffff8040000000
shim: DTB       0x40243000-0x40344000
shim: root task 0x40344000-0x40380000 vaddr 0x400000 entry 0x400000
shim: entering seL4
Bootstrapping kernel
...
Booting all finished, dropped to user space
hello from the root task
boot info: node 0 of 1, 53 untyped caps
  untyped 0: paddr 0x0 size 2^27 device
...
sched control caps: 278-278
scheduling context: budget 5000 us per 5000 us, 12897 us used
disk: virtio-blk, 4096 blocks
EXEC.EXE: started at 0x40010b68, 23 of 1024 pages in use
%EXEC-I-START, vaxpunk executive, free pages: 00000369
%MOUNT-I-MOUNTED, VAXPUNK mounted on _DKA0:
$
```

The system disk, `DKA0:`, is a Files-11 ODS-2 volume, `out/sysdisk.img`,
which the build makes with `ods` and QEMU attaches read only
([ADR-0007](docs/adr/0007-system-disk-files-11-and-rms.md)). The root task
reads `EXEC.EXE` from it, and the executive mounts it and reads files with
RMS. `ods dir out/sysdisk.img '[...]'` lists it on the host.

The `$` is DCL's prompt, on the console's process, `SYSTEM`. `RUN image`
runs an image from `[SYSEXE]` (`.EXE` is the default type), `DIRECTORY`
lists files, in the default directory unless told where (`DIR`, `DIR
[SYSEXE]P%NG`, `DIR [000000]`), `SET DEFAULT [dir]` (or `[-]`, `[.dir]`) and
`SHOW DEFAULT` set and show that, `[SYSMGR]` at first, `TYPE file` writes a text file (`TYPE WELCOME.TXT`),
`EDIT file` looks at one with EDT's line mode, read only (`HELP` at its
`*` prompt),
`DEFINE name equivalence`, `DEASSIGN name` and `SHOW LOGICAL name` make,
delete and translate logical names (`SHOW LOGICAL SYS$INPUT`, or `SHOW
LOGICAL` alone to list them all), `COPY` and `DELETE` copy and delete
files, `INITIALIZE` and `MOUNT` make and mount a volume on the ramdisk,
`MDA0:`, the one disk they can write
([ADR-0009](docs/adr/0009-ramdisk-writable-files-11.md)), `HELP` lists
the commands and `LOGOUT` ends the process:

```
$ DIR [SYSEXE]

Directory DKA0:[SYSEXE]

COPY.EXE;1          DCL.EXE;1           DELETE.EXE;1        DIRECTORY.EXE;1
EDIT.EXE;1          EXEC.EXE;1          HOG.EXE;1           INIT.EXE;1
MOUNT.EXE;1         NUDGE.EXE;1         PING.EXE;1          PONG.EXE;1
SLEEPER.EXE;1       SNOOP.EXE;1         STARTUP.EXE;1       SVCTEST.EXE;1
TIMETEST.EXE;1      TYPE.EXE;1          USURP.EXE;1

Total of 19 files.
$ INITIALIZE MDA0: RAM
$ MOUNT MDA0: RAM
%MOUNT-I-MOUNTED, RAM mounted on _MDA0:
$ COPY [SYSMGR]WELCOME.TXT MDA0:[000000]
$ DIR MDA0:[000000]WELCOME.TXT;*

Directory MDA0:[000000]

WELCOME.TXT;1

Total of 1 file.
$ DELETE MDA0:[000000]WELCOME.TXT;1
```

`RUN STARTUP` starts the programs that put the executive's services to
work:

```
$ RUN STARTUP
STARTUP: $EXPREG made 4 pages, and they hold what I wrote, at 00031000
SLEEPER: hibernating until I'm deleted
STARTUP: created SLEEPER, which ran first, PID 00030003
STARTUP: created PING and PONG; waiting until PONG wakes me
PING 00000001
  PONG 00000001
PING 00000002
  PONG 00000002
PING 00000003
  PONG 00000003
PONG: woke STARTUP, exiting
PING: done, returning
STARTUP: woken, deleting SLEEPER
STARTUP: done, SVCTEST, HOG and TIMETEST next
$ TIMETEST: the system time's high longword is 00BC3534
SLEEPER: hibernating until I'm deleted
HOG: NUDGE ran while I computed: preempted at quantum end
SVCTEST: ok
%EXEC-W-EXITED, process NOSUCH exited with status 00000910
%SYSTEM-F-ACCVIO, access violation, virtual address 40010000, PC 00010020, process SNOOP
%EXEC-W-EXITED, process SNOOP exited with status 0000000C
%SYSTEM-F-OPCDEC, reserved instruction at PC 00010030, process USURP
%EXEC-W-EXITED, process USURP exited with status 0000043C
TIMETEST: ok
```

STARTUP runs in `SYSTEM`, and DCL prompts again once it returns, while
the processes it created go on. Between commands the CPU idles, taking a
clock interrupt every 10 ms. There is no CTRL/Y yet: an image that never
exits, such as `SLEEPER`, keeps the console until QEMU is stopped. `just
check` boots the same way without a terminal, types `RUN STARTUP`, `RUN
SNOOP`, a bad verb, `DIR [SYSEXE]P%NG`, `TYPE WELCOME.TXT`, an `EDIT
WELCOME.TXT` session, and a logical
name's `DEFINE`, `SHOW LOGICAL` and `DEASSIGN`, `SHOW LOGICAL` alone, and
`SET DEFAULT` and `SHOW DEFAULT`, and a round trip through the ramdisk,
prints the executive's part and fails unless the processes ran to the end.

EDK2 prints a few `Error: Image at ... start failed` and `Tpm2...` lines
before Limine starts. That is normal for the firmware QEMU ships. Don't
type before DCL's prompt: EDK2 and Limine read keys too.

Day to day: edit `roottask/` (`src/` for the PAL, `exec/` for the
executive, `sysexe/` for the programs on the system disk, `sysmgr/` for
its text files), then `cargo run -p boot`. Only the root task and the
system disk are rebuilt and the ESP image re-stitched. On an M3, the root task prints
about one second after the command.

## Layout

| Directory | What | Output |
| --- | --- | --- |
| `kernel/` | seL4 16.0.0 (submodule), built with its own CMake | `kernel.elf`, libsel4's headers, `platform_gen.json` |
| `shim/` | Limine-protocol program that loads seL4 and the root task; see [shim/README.md](shim/README.md) | `shim.elf` |
| `roottask/` | the root task, the PAL, in freestanding C (`src/`); the MACRO-32 executive (`exec/`) and the programs it runs (`sysexe/`), which `build.rs` compiles and links with the vtools crates and writes, with `sysmgr/`'s text, to the system disk with `ods-image` | `roottask.elf`, `sysdisk.img` |
| `image/` | Limine config, the ESP builder (mtools) and `boot`, which copies the three ELFs and `sysdisk.img` to `out/`, stitches the ESP and runs QEMU | `out/esp.img` |
| `scripts/` | host setup, Limine download, QEMU wrapper and console filter | `out/serial.log` |
| `ods/` | Files-11 ODS-2/ODS-5 file system in Rust: the library, the `ods` CLI and a FUSE mount; see [ods/README.md](ods/README.md) | `target/` |
| `vtools/` | VMS-style toolchain in Rust: the `vasm` assembler, the `vmacro` MACRO-32 compiler, `vlink` linker and `vlib` librarian, object, library and image formats, `vdump` to inspect them, and `vrun`, which runs images in QEMU; see its [PRD](docs/prd/0001-vtools.md) | `target/` |
| `docs/` | ADRs, PRDs and design documents, numbered per kind; see [docs/README.md](docs/README.md) | |

The whole repository is one Cargo workspace. `kernel/`, `shim/` and
`roottask/` are crates whose `build.rs` runs the component's C build (seL4's
CMake, or gcc) into Cargo's `OUT_DIR`; `cargo build -p <name>` builds one
with what it needs. The components share nothing but those output files.
The kernel hands `shim` and `roottask` its libsel4 headers,
`platform_gen.json` and toolchain prefix (`links = "sel4"`), and each crate
exports its ELF's path as `ELF` for `boot`.

- `kernel/config.cmake` sets `KernelIsMCS`, and the root task refuses to
  build against a non-MCS libsel4. Code written for the classic API needs
  the MCS forms: `seL4_Recv`, `seL4_NBRecv` and `seL4_ReplyRecv` take a reply
  object cap, `seL4_Reply` and `seL4_CNode_SaveCaller` are gone, and a new
  thread runs only once it is bound to a configured scheduling context
  (`seL4_SchedControl_Configure`, `seL4_SchedContext_Bind`).
- The kernel rebuilds only when `kernel/config.cmake`, `kernel/qemu.env`,
  `kernel/requirements.txt`, `CROSS_COMPILE` or a file in `kernel/seL4`
  change, and the shim and root task rebuild with it.
- `kernel/qemu.env` holds the QEMU CPU, RAM and GIC version. seL4 compiles in
  that machine's memory map, and `scripts/run-qemu.sh` reads the same file.
- Any root task can replace `roottask.elf`: it must be a static AArch64 ELF
  whose first segment is page aligned. seL4 calls its entry point with the
  boot info pointer in `x0`.
- The root task drives the system disk itself: a virtio-blk device on QEMU
  virt's virtio-mmio transports (from `0x0a000000`), polled, in modern
  mode, which `scripts/run-qemu.sh` asks for with
  `virtio-mmio.force-legacy=false`. The ESP stays on PCI, for EDK2.
- The root task maps `EXEC.EXE`'s sections at their link addresses (vlink's
  default base, 0x10000) in a new address space, with fresh frames from the
  largest RAM untyped, and starts a thread there at the transfer address.
  That thread, the executive, has no capabilities: its privileged
  instructions trap to the root task, which is its fault handler
  ([DESIGN-0001](docs/design/0001-pal-interface.md)). So do its processes'
  threads, which the root task makes when the executive first switches to
  them. vrun's `SVC` calls aren't there.
- Pins: seL4 by submodule commit (tag 16.0.0), Limine 11.4.1 by version and
  SHA-256 in `scripts/fetch-limine.sh`. The EDK2 firmware comes from the
  QEMU install (`EDK2_FW=` overrides it).
- The Rust projects, `ods/` and `vtools/`, are the workspace's default
  members, so a plain `cargo test` tests both without the C toolchain;
  `cargo test -p 'ods*'` or `cargo test -p 'v*'` tests one. `ods-fuse` needs
  FUSE (fuse3 on Linux, macFUSE on macOS). vtools's tests run images under
  `vrun` in QEMU; `VRUN_FLAGS=--hvf cargo test -p 'v*'` runs them under HVF.

## Toolchain

The kernel's `build.rs` picks the compiler with `CROSS_COMPILE` (default:
`aarch64-elf-` if installed, else `aarch64-linux-gnu-`), and the shim and root
task build with the same one, for example
`CROSS_COMPILE=aarch64-none-elf- cargo run -p boot`. `vrun`'s boot stub is
assembled with the same `CROSS_COMPILE` binutils.

## Debugging

`cargo run -p boot -- --gdb` starts QEMU halted with a GDB server on port
1234. In another terminal:

```sh
lldb out/roottask.elf -o 'gdb-remote 1234' -o 'b main' -o c
gdb-multiarch out/roottask.elf -ex 'target remote :1234' -ex 'b main' -ex c
```

`scripts/run-qemu.sh` takes the same flags and boots the last `out/esp.img`
without building.

`cargo run -p boot -- --hvf` runs under Hypervisor.framework instead of TCG.
It boots the same kernel, which is why `kernel/qemu.env` picks GICv3 (HVF
does not emulate GICv2). This is best effort: the kernel is built for a
Cortex-A57 while HVF offers only `-cpu host`, and seL4 warns that the
counter runs at 24 MHz instead of the 62.5 MHz it was built for.

`cargo run -p boot -- --uart1` serves the second serial port on telnet
`localhost:4444` (`--uart1=PORT` picks another port, for QEMUs in parallel
worktrees). Connect with `telnet localhost 4444` after setting `mode
character`, or `socat -,raw,echo=0 tcp:localhost:4444`. The port is a PL011
at `0x09040000` on SPI 8 (GIC IRQ 40), alias `serial1` in QEMU's DTB. It is
always there, so the guest sees the same machine with or without the flag;
without it the port goes nowhere. Nothing in the guest drives it yet.
