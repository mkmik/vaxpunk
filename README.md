# vaxpunk

OpenVMS remake on arm64.

vaxpunk boots on QEMU aarch64 through UEFI into `EXEC.EXE`, the executive,
an image compiled from MACRO-32 with vtools. The executive manages memory,
and creates, schedules and deletes processes, each running an image from
the system disk, which take turns on one CPU and synchronize with IPL and
event flags ([ADR-0003](docs/adr/0003-one-cpu-many-threads.md),
[DESIGN-0002](docs/design/0002-executive-processes.md)). The executive
reaches the processor through VAX privileged instructions, served by a PAL
below it ([ADR-0002](docs/adr/0002-root-task-is-the-pal.md),
[DESIGN-0001](docs/design/0001-pal-interface.md)).

Today the PAL is the root task of the seL4 microkernel, which vaxpunk uses
to emulate the Alpha's four access modes, kernel, executive, supervisor and
user, on an ARM64 CPU that offers only EL0 and EL1 to an OS: each mode of a
process is a thread with an address space of its own
([ADR-0005](docs/adr/0005-access-modes-are-threads.md)). seL4 may give way
later to a more native approach. It is built with its MCS
(mixed-criticality scheduling) API: threads run on scheduling contexts with
a budget and period, and IPC replies go through reply objects.

```
EDK2 -> Limine (BOOTAA64.EFI) -> shim -> seL4 (kernel.elf) -> root task (roottask.elf) -> EXEC.EXE -> processes
```

## Host setup (once)

macOS (Apple Silicon, Homebrew) or Debian/Ubuntu (x86_64 or arm64):

```sh
scripts/setup-host.sh
```

It installs an aarch64 bare-metal C compiler, `cmake`, `ninja`, `dtc`, `uv`,
`mtools`, QEMU with its EDK2 firmware, `just`, and checks out the seL4 and lwIP submodules.
Cargo drives the whole build, so you also need a Rust toolchain, for example
from [rustup](https://rustup.rs).

## Build and run

```sh
cargo run -p boot
```

The first run builds seL4 (about 10 seconds), downloads Limine and boots QEMU.
Quit QEMU with `Ctrl-A x`, or `LOGOUT`, which halts the system and powers
QEMU off. EDK2 and Limine clear the console and move the
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
disk 0: virtio-blk, 4096 blocks
disk 1: virtio-blk, 4096 blocks
EXEC.EXE: started at 0x40010b68, 23 of 1024 pages in use
tcpip: lwIP 2.2.1 on virtio-net, MAC 52:54:0:12:34:56
%EXEC-I-START, vaxpunk executive, free pages: 00000369
%MOUNT-I-MOUNTED, VAXPUNK mounted on _DKA0:
%SYSTEM-W-NOHOMEBLK, Files-11 home block not found on volume
$
```

The system disk, `DKA0:`, is a Files-11 ODS-2 volume, `out/sysdisk.img`,
which the build makes with `ods` and QEMU attaches read only
([ADR-0007](docs/adr/0007-system-disk-files-11-and-rms.md)). The root task
reads `EXEC.EXE` from it, and the executive mounts it and reads files with
RMS. `ods dir out/sysdisk.img '[...]'` lists it on the host. The data
disk, `DKB0:`, is another, `out/datadisk.img`, which QEMU attaches
read-write and `run-qemu.sh` makes, blank, the first time
([ADR-0012](docs/adr/0012-data-disk-writable-files-11.md)). Once
`INITIALIZE DKB0: label` has written a volume there, each boot mounts it,
with the `MOUNT DKB0:` in `SYS$MANAGER:SYSTARTUP_VMS.COM`, which says
`NOHOMEBLK` until then, and what you put there stays; delete the file to
start afresh. `SHOW
DEVICES` lists the disks and what is mounted on them:

```
$ SHOW DEVICES

Device                  Device           Error    Volume         Free
 Name                   Status           Count     Label        Blocks
DKA0:                   Mounted wrtlck       0  VAXPUNK           3560
DKB0:                   Mounted              0  DATA              4025
MDA0:                   Mounted              0  RAM                954

Device                  Device           Error
 Name                   Status           Count
OPA0:                   Online               0
```

The `$` is DCL's prompt, on the console's process, `SYSTEM`. `RUN image`
runs an image from `SYS$SYSTEM:` (`.EXE` is the default type), `DIRECTORY`
lists files, in the default directory unless told where (`DIR`, `DIR
[SYSEXE]P%NG`, `DIR [000000]`, `DIR SYS$SYSTEM:`; a device may be a
logical name), `SET DEFAULT [dev:][dir]` (or `[-]`, `[.dir]`) and
`SHOW DEFAULT` set and show that, `DKA0:[SYSMGR]` at first, `SHOW DEVICES`
lists the devices, `TYPE file` writes a text file (`TYPE WELCOME.TXT`),
`EDIT file` edits one with EDT: its line mode types and changes lines,
`CHANGE` at its `*` prompt goes to keypad mode, on the screen, and
`EXIT` writes a new version, on the ramdisk (`HELP` at the prompt, PF2
on the screen),
`DEFINE name equivalence`, `DEASSIGN name` and `SHOW LOGICAL name` make,
delete and translate logical names (`SHOW LOGICAL SYS$INPUT`, or `SHOW
LOGICAL` alone to list them all), `SHOW PROCESS` and `SHOW SYSTEM` show
the process and list them all, `TCPIP` sets and shows the network's
settings and the hosts it knows by name (*Networking*), `SET HOST
host` logs in to another vaxpunk, `COPY` and `DELETE` copy and delete
files, `BACKUP` saves them in a save set VMS's BACKUP reads, and lists and
restores one, `INITIALIZE`, `MOUNT` and `DISMOUNT` make, mount and dismount a
volume on the data disk, `DKB0:`, or the ramdisk, `MDA0:`, which
`SYSTARTUP_VMS.COM` makes and mounts at boot, the disks they can write ([ADR-0009](docs/adr/0009-ramdisk-writable-files-11.md)), where
`CREATE/DIRECTORY [A.B]` makes directories, `CREATE file` a text file
of the lines typed up to CTRL/Z, `SET COMMAND file` adds the
verbs a `.CLD` file defines (`SET COMMAND SYS$MANAGER:DCLTEST`, then
`GREET world`), `name := $image` makes a foreign command, `HELP` lists
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
clock interrupt every 10 ms. CTRL/Y stops an image that never exits,
such as `SPIN` or `SLEEPER`, and gives the `$` prompt back; `CONTINUE`
goes on with it. `cargo test -p boot` boots the same way without a terminal,
types `RUN STARTUP`, `RUN SNOOP`, a bad verb, `DIR [SYSEXE]P%NG`, `TYPE
WELCOME.TXT`, an `EDIT WELCOME.TXT` session, and a logical
name's `DEFINE`, `SHOW LOGICAL` and `DEASSIGN`, `SHOW LOGICAL` alone, and
`SET DEFAULT` and `SHOW DEFAULT`, a round trip through the ramdisk with
a keypad-mode `EDIT` on it, a
`COPY` to the data disk, made afresh, which `ods` then checks on the host,
`SHOW PROCESS` and `SHOW SYSTEM`, `SET TERMINAL` and `SHOW TERMINAL`,
stops `SPIN` and `SLEEPER` with CTRL/Y and continues them, prints the
executive's part and fails unless the processes ran to the end.

EDK2 prints a few `Error: Image at ... start failed` and `Tpm2...` lines
before Limine starts. That is normal for the firmware QEMU ships. Don't
type before DCL's prompt: EDK2 and Limine read keys too.

Day to day: edit `vms/` (`exec/` for the executive, `sysexe/` for the
programs on the system disk, `sysmgr/` for its text files) or `pal/src/`
for the PAL, then `cargo run -p boot`. Only the root task and the
system disk are rebuilt and the ESP image re-stitched. On an M3, the root task prints
about one second after the command.

## Layout

| Directory | What | Output |
| --- | --- | --- |
| `vms/` | the OS above the PAL, with nothing of seL4 in it: the MACRO-32 and BLISS executive (`exec/`), the programs it runs (`sysexe/`), DCL's command tables (`cld/`) and SYSTEM's text files (`sysmgr/`), which `build.rs` compiles and links with the vtools crates and writes to the system disk with `ods-image`; see [vms/README.md](vms/README.md) | `sysdisk.img` |
| `pal/` | everything that knows seL4: the root task, which is the PAL the executive calls with privileged instructions, in freestanding C (`src/`), and the components under it; see [pal/README.md](pal/README.md) | `roottask.elf` |
| `pal/kernel/` | seL4 16.0.0 (submodule), built with its own CMake | `kernel.elf`, libsel4's headers, `platform_gen.json` |
| `pal/shim/` | Limine-protocol program that loads seL4 and the root task; see [pal/shim/README.md](pal/shim/README.md) | `shim.elf` |
| `pal/tcpip/` | the TCP/IP component the root task starts below the executive: lwIP (submodule) with a virtio-net driver and the port adapter, in freestanding C; see [DESIGN-0003](docs/design/0003-tcpip-port.md) | `tcpip.elf` |
| `boot/` | Limine config, the ESP builder (mtools) and `boot`, which copies the three ELFs and `sysdisk.img` to `out/`, stitches the ESP and runs QEMU; see [boot/README.md](boot/README.md) | `out/esp.img` |
| `scripts/` | host setup, Limine download, QEMU wrapper and console filter | `out/serial.log` |
| `crosstools/` | the host tools that build vaxpunk and its disks; see [crosstools/README.md](crosstools/README.md) | |
| `crosstools/ods/` | Files-11 ODS-2/ODS-5 file system in Rust: the library, the `ods` CLI and a FUSE mount; see [crosstools/ods/README.md](crosstools/ods/README.md) | `target/` |
| `crosstools/vtools/` | VMS-style toolchain in Rust: the `vasm` assembler, the `vmacro` MACRO-32 compiler, `vlink` linker and `vlib` librarian, object, library and image formats, `vdump` to inspect them, and `vrun`, which runs images in QEMU; see [crosstools/vtools/README.md](crosstools/vtools/README.md) and its [PRD](docs/prd/0001-vtools.md) | `target/` |
| `docs/` | ADRs, PRDs and design documents, numbered per kind; see [docs/README.md](docs/README.md) | |

The whole repository is one Cargo workspace. `pal/kernel/`, `pal/shim/`,
`pal/tcpip/` and `pal/` are crates whose `build.rs` runs the component's C
build (seL4's CMake, or gcc) into Cargo's `OUT_DIR`, and `vms/`'s builds
the system disk with the vtools crates, no C toolchain needed;
`cargo build -p <name>` builds one with what it needs. The components share
nothing but those output files.
The kernel hands `shim`, `tcpip` and `pal` its libsel4 headers,
`platform_gen.json` and toolchain prefix (`links = "sel4"`), and each crate
exports its ELF's path as `ELF` for `boot`.

- `pal/kernel/config.cmake` sets `KernelIsMCS`, and the root task refuses to
  build against a non-MCS libsel4. Code written for the classic API needs
  the MCS forms: `seL4_Recv`, `seL4_NBRecv` and `seL4_ReplyRecv` take a reply
  object cap, `seL4_Reply` and `seL4_CNode_SaveCaller` are gone, and a new
  thread runs only once it is bound to a configured scheduling context
  (`seL4_SchedControl_Configure`, `seL4_SchedContext_Bind`).
- The kernel rebuilds only when `pal/kernel/config.cmake`, `pal/kernel/qemu.env`,
  `pal/kernel/requirements.txt`, `CROSS_COMPILE` or a file in `pal/kernel/seL4`
  change, and the shim and root task rebuild with it.
- `pal/kernel/qemu.env` holds the QEMU CPU, RAM and GIC version. seL4 compiles in
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
- Pins: seL4 by submodule commit (tag 16.0.0), lwIP likewise (tag
  STABLE-2_2_1_RELEASE), Limine 11.4.1 by version and
  SHA-256 in `scripts/fetch-limine.sh`. The EDK2 firmware comes from the
  QEMU install (`EDK2_FW=` overrides it).
- The host tools in `crosstools/` and the system disk in `vms/` are the
  workspace's default members, so a plain `cargo test` builds and tests
  them without the C toolchain;
  `cargo test -p 'ods*'` or `cargo test -p 'v*'` tests one. `ods-fuse` needs
  FUSE (fuse3 on Linux, macFUSE on macOS). vtools's tests run images under
  `vrun` in QEMU; `VRUN_FLAGS=--hvf cargo test -p 'v*'` runs them under HVF.

## Toolchain

The kernel's `build.rs` picks the compiler with `CROSS_COMPILE` (default:
`aarch64-elf-` if installed, else `aarch64-linux-gnu-`), and the shim and root
task build with the same one, for example
`CROSS_COMPILE=aarch64-none-elf- cargo run -p boot`. `vrun`'s boot stub is
assembled with the same `CROSS_COMPILE` binutils.

## Networking

QEMU has a virtio-net device on its user network, where the guest is
`10.0.2.15/24`, the host `10.0.2.2`, and a DHCP server gives the guest
those. The root task starts the TCP/IP component, lwIP, beside the
executive, which talks to it through a port of shared pages and drives
it as `BGA0:` ([DESIGN-0003](docs/design/0003-tcpip-port.md)). Programs
make TCP, UDP and raw ICMP sockets on it as TCP/IP Services' do, with
`$QIO` on `TCPIP$DEVICE:`, item lists and ioctls
([ADR-0024](docs/adr/0024-sockets-have-tcpip-services-qio-interface.md)). At boot
`TCPIP START COMMUNICATION` sets the interface, `WE0`, from the saved
settings, or from DHCP if none are saved. TCP/IP Services' `TCPIP`
utility changes it ([ADR-0022](docs/adr/0022-tcpip-utility-and-dhcp.md)),
from DHCP or by hand:

```
$ TCPIP SET INTERFACE WE0 /DHCP
$ TCPIP
TCPIP> SET INTERFACE WE0 /HOST=10.0.2.15 /NETWORK_MASK=255.255.255.0
TCPIP> SET ROUTE /DEFAULT /GATEWAY=10.0.2.2
TCPIP> SHOW INTERFACE
Interface  IP_Addr          Network mask     Gateway          Link
 WE0       10.0.2.15        255.255.255.0    10.0.2.2         up
TCPIP> PING 10.0.2.2 /NUMBER_PACKETS=2
PING 10.0.2.2 (10.0.2.2): 56 data bytes
64 bytes from 10.0.2.2: icmp_seq=0 ttl=255 time=0 ms
64 bytes from 10.0.2.2: icmp_seq=1 ttl=255 time=0 ms
----10.0.2.2 PING Statistics----
2 packets transmitted, 2 packets received, 0% packet loss
TCPIP> EXIT
```

`PING` without `/NUMBER_PACKETS` goes on until CTRL/C.

`SET INTERFACE` and `SET ROUTE` change the running system only. `SET
CONFIGURATION INTERFACE WE0`, with `/DHCP` or `/HOST` and
`/NETWORK_MASK`, and `SET ROUTE /DEFAULT /GATEWAY=address /PERMANENT`
save the settings, as TCP/IP Services split them, and `TCPIP START
COMMUNICATION` sets them at boot. They are saved on the ramdisk, `MDA0:`,
which `SYSTARTUP_VMS.COM` makes afresh at every boot, so a fixed address
is a pair of those commands in that procedure; without them the
interface asks DHCP. `SET HOST name /ADDRESS=address [/ALIAS=(...)]`,
`SET NOHOST name` and `SHOW HOST [name]` keep TCP/IP Services' hosts
database, `TCPIP$HOST.DAT`, there too, and every command that takes a
host takes a name or an alias from it
([ADR-0025](docs/adr/0025-hosts-database.md)). Every
system with a network runs `TCPIP$TELNET`, which takes `SET HOST`
and `TELNET` logins on TCP port 23: `SET HOST 10.0.2.15` logs in to the
system itself. `TELNET address port`, or `/PORT=port`, as TCP/IP
Services takes it, talks to any other port a line at a time, a poor
man's netcat; CTRL/Z hangs up.

`COPY/HTTP` is a poor man's curl, with VMS's remote file syntax, the
server a node and the path in quotes: `COPY/HTTP 10.0.2.2::"/index.html"
[]`, or a whole URL after a node that is only a name, `COPY/HTTP
URL::"http://10.0.2.2:8080/pub/notes.txt" DKB0:[000000]`. The page goes,
byte for byte, binary or not, into a new STREAM_LF file, as VMS's
ports of curl and wget write one, named after the path's last part
where the output doesn't say (`NOTES.TXT`, `INDEX.HTML` for a path
ending in `/`); 404 is `%RMS-E-FNF`, 401 and 403 `%RMS-E-PRV`, another
error `%RMS-F-NETFAIL`. Names come from the hosts database only, since
there is no DNS yet, and plain HTTP: an `https` URL is `%RMS-F-SUPPORT`.

`run-qemu.sh` reads `NETDEV`, QEMU's `-netdev` for the network, and
`MAC`, `LOG` and `DATADISK`, so that a second vaxpunk can share a network
with the first, out of the same `out/`. Two on a socket network:

```sh
LOG=out/a.log DATADISK=out/a-data.img MAC=52:54:00:00:00:0a \
  NETDEV=socket,id=net0,listen=127.0.0.1:12345 scripts/run-qemu.sh
LOG=out/b.log DATADISK=out/b-data.img MAC=52:54:00:00:00:0b \
  NETDEV=socket,id=net0,connect=127.0.0.1:12345 scripts/run-qemu.sh
```

Give them addresses, say `10.0.0.1` and `10.0.0.2`, and `SET HOST
10.0.0.1` on the second. `NETDEV=user,id=net0,hostfwd=tcp::2323-:23`
lets the host in, line at a time (`nc localhost 2323`). `cargo test -p
boot --test network` does all this.

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
It boots the same kernel, which is why `pal/kernel/qemu.env` picks GICv3 (HVF
does not emulate GICv2). This is best effort: the kernel is built for a
Cortex-A57 while HVF offers only `-cpu host`, and seL4 warns that the
counter runs at 24 MHz instead of the 62.5 MHz it was built for.
`just boot` adds `--hvf` on a Mac.

`cargo run -p boot -- --uart1` serves the second serial port on telnet
`localhost:4444` (`--uart1=PORT` picks another port, for QEMUs in parallel
worktrees). Connect with `telnet localhost 4444` after setting `mode
character`, or `socat -,raw,echo=0 tcp:localhost:4444`. The port is a PL011
at `0x09040000` on SPI 8 (GIC IRQ 40), alias `serial1` in QEMU's DTB. It is
always there, so the guest sees the same machine with or without the flag;
without it the port goes nowhere. Nothing in the guest drives it yet.
