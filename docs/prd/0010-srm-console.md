# PRD-0010 — An SRM console: the `>>>` prompt before VMS

Oct 16, 2026 · @Marko Mikulicic

## Context and goal

vaxpunk gets a pre-boot console modeled on the Alpha SRM console: the `>>>`
prompt the machine sits at before VMS runs, and returns to when VMS halts
or crashes. It owns four jobs: initialize the machine, talk to the
operator, boot VMS (and restart it), and hold the settings that decide what
boots and how.

It is the first piece of vaxpunk the operator sees, and it is meant to
grow: today it hands off to VMS; later it is where the PAL lives and where
the machine-level duties of the substrate move to. This PRD covers what
the console looks like and how it behaves. How it is built on the current
substrate is out of scope here. On the map it is
[t8y:/platform/srm-console](../map/platform/srm-console.md).

### Background: the Alpha SRM console

On Alpha, the SRM console was the firmware every machine running OpenVMS
or DIGITAL UNIX needed. Windows NT on Alpha used a different firmware
(ARC, later AlphaBIOS); a variable chose which one came up at the next
power cycle. ([DIGITAL SRM console guide](https://ente.limmat.ch/ftp/pub/alpha/digital_server_7000/doc/srm/srmcons.pdf))

The guide lists three service jobs: power-up diagnostics and
initialization, the operator interface, and bootstrapping and restarting
the operating system.

Under the prompt, SRM was a small operating system of its own:

- An event-driven executive with processes, memory management, SMP
  support, a memory-resident file system, and its own interrupt and error
  handling.
- Unix-style I/O: devices and files are byte streams, reachable locally on
  a serial port or remotely over Ethernet.
- Class drivers for SCSI, FAT, MOP and BOOTP, and port drivers for
  Ethernet and SCSI controllers, NVRAM, flash, memory and registers.
- A shell with line editing and recall, online help, diagnostics and
  exercisers, and "script creation and simple editing".

In daily use it did a few things: show what hardware and boot devices
exist, keep boot settings in NVRAM, boot VMS with chosen flags
(conversational boot, alternate root), run diagnostics, and inspect a
stopped machine's memory and registers. It also supplied the PALcode the
OS ran on; `show pal` printed its version.

### Goals

- Feel like an Alpha SRM console to someone who used one: same prompt,
  command names, variable names, device names and boot flow.
- Choose and configure the boot device, and auto-boot a default after
  power-up, halt or crash.
- Keep settings across reboots in emulated NVRAM.
- Give an operator a place to land when VMS halts or crashes, inspect the
  stopped machine, and continue, crash-dump or reboot.
- Hand VMS a defined set of boot parameters, so VMS never needs to know
  what substrate it runs on.
- Start whatever must run before VMS (today: the networking sidecar) from
  the console's power-up script.
- Be small, and written in the project's own languages, so it can grow
  into the layer that owns the machine.

## Non-goals

For now:

- Firmware update, ARC/AlphaBIOS switching, ISA configuration, and other
  board-specific utilities of real SRM.
- Network boot (MOP, BOOTP) and DECnet anything.
- A graphics console; serial only.
- A full diagnostic suite. A small memory test is enough at first.
- Running real Alpha SRM firmware or exact byte compatibility with its
  NVRAM format.
- How the console is built on the current microkernel substrate; covered
  in a separate document.

## Users and use cases

There is one user: the operator at the serial console, usually Marko,
sometimes a script driving QEMU.

| Use case | What happens at the console |
| --- | --- |
| Unattended boot | Power on, the power-up script runs, `auto_action` is `boot`, VMS comes up with no keys pressed |
| Pick another disk | Interrupt auto-boot, `show device`, `boot dka100` |
| Conversational boot | `boot -flags 0,1` to stop in SYSBOOT and change system parameters |
| Alternate root | `boot -flags 1,0` to boot system root SYS1 instead of SYS0 |
| Change the default | `set bootdef_dev dka100`, `set auto_action boot`; kept across power cycles |
| VMS halted | Machine drops to `>>>`; operator examines registers or memory, then `continue`, `crash` or `boot` |
| Debug early boot | `boot -halt` loads the image, stops before jumping to it, operator inspects, then `continue` |
| Check the machine | `show memory`, `show config`, `show version`, `show pal`, a quick `test` |
| Scripted test runs | A harness sends commands over the serial line and waits for the prompt |

The flag values follow VMS: the first number is the system root, the
second the boot flags (bit 0 = conversational).

## Power-up, halt and restart

The machine is always in one of three places: at the console prompt,
booting, or running VMS. `auto_action` (`halt`, `boot` or `restart`) picks
what happens after power-up, a halt or an error
([DIGITAL SRM guide](https://ente.limmat.ch/ftp/pub/alpha/digital_server_7000/doc/srm/srmcons.pdf)).

```
 power-up ──> nvram script ──> auto_action ─┬─ halt ────────────────────────> >>>
                                            └─ boot, restart ──> countdown ──> boot
                                                                   │ Ctrl/C
                                                                   └────────> >>>

 >>> ── boot ──> boot ──> VMS running         >>> ── continue ──> VMS running

 VMS running ── Ctrl/P, VMS halt, fatal error ──> auto_action ─┬─ halt ────> >>>
                                                               ├─ boot ────> boot
                                                               └─ restart ─> dump (after a crash) ──> boot
```

With `restart`, a crash first writes a dump before rebooting. Ctrl/C
during the auto-boot countdown also stops at `>>>`, and a halt assertion
at power-up skips both the `nvram` script and the auto-boot once.

## Command set

vaxpunk ships the commands an OpenVMS operator used, with real SRM's names
and syntax. Board utilities (`lfu`, `isacfg`, `prcache`, `alphabios`) are
left out. Sources: the
[DS10 console reference](https://metebalci.com/blog/alpha-and-tru64-in-2023/ds10-console-reference.pdf)
command summary and the
[DIGITAL SRM guide](https://ente.limmat.ch/ftp/pub/alpha/digital_server_7000/doc/srm/srmcons.pdf).

| Command | What it does | Phase |
| --- | --- | --- |
| `boot [-file f] [-flags r,f] [-halt] [dev...]` | Load and start VMS; defaults from `bootdef_dev`, `boot_file`, `boot_osflags`; options override for this boot only | 1 |
| `show <envar>` / `show *` / `show boot*` | Print variables, with wildcards | 1 |
| `set <envar> <value>`, `set -default <envar>` | Change or reset a variable; unknown names create a user variable | 1 |
| `clear <envar>` | Reset a variable to its default | 1 |
| `show device [name]` | List boot-capable devices with long and short names | 1 |
| `show memory`, `show config`, `show version`, `show pal` | Describe the machine, console version and PAL version | 1 |
| `initialize` (`init`) | Reset the console and machine to a known state; new variable values take effect | 1 |
| `help [command]` | Online help | 1 |
| `examine` / `deposit` (`e` / `d`) | Read or write memory (`pmem:`, `vmem:`) and registers; sizes `-b -w -l -q`; `-n` count; `+ - * @` addressing | 2 |
| `continue` | Resume VMS after a halt | 2 |
| `crash [dev]` | Restart VMS into its crash-dump path | 2 |
| `halt` / `stop` | Halt a CPU (meaningful only with SMP) | 3 |
| `start <addr>` | Start execution at an address | 3 |
| `edit <file>` | Line editor for RAM files and the `nvram` script | 2 |
| `more`, `cat`, `ls` | Read console files, including the event log | 2 |
| `set password`, `clear password`, `set secure`, `login` | Console security | 3 |
| `test [-t secs]`, `memtest` | Built-in diagnostics | 3 |
| `ps`, `kill`, `show_status`, `kill_diags` | Console process list for background jobs | 3 |
| `sleep`, `echo` | Small helpers for scripts | 2 |

Command-line rules follow real SRM:

- Lines up to 255 characters; `\` at the end continues the line with a
  `_>` prompt.
- Keywords can be shortened to any unambiguous prefix (`b` for `boot`,
  `sho dev`).
- Options start with a dash; case doesn't matter; `#` starts a comment.
- Addresses and `deposit` data are hex, other numbers decimal; `%d`, `%x`,
  `%o`, `%b` override the radix.
- Line editing: Ctrl/A insert or overstrike, Ctrl/B or up-arrow recall
  (16 lines), Ctrl/E end, Ctrl/H start, Ctrl/U kill line, Ctrl/R retype.
- Ctrl/C kills the foreground command; Ctrl/O, Ctrl/S, Ctrl/Q control
  output; Ctrl/P halts the machine.

## Environment variables and NVRAM

Variables carry configuration between the console and VMS, and the
persistent ones live in emulated NVRAM. On real SRM a changed value takes
effect after `init` or a reset; vaxpunk keeps that rule.
([DS10 console reference](https://metebalci.com/blog/alpha-and-tru64-in-2023/ds10-console-reference.pdf))

| Variable | Values | Default | Persists | Meaning |
| --- | --- | --- | --- | --- |
| `auto_action` | `halt`, `boot`, `restart` | `halt` | yes | What happens after power-up, halt or error |
| `bootdef_dev` | device or comma list | empty | yes | Default boot device; a list is tried in order |
| `boot_osflags` | `root,flags` (hex) | `0` | yes | System root and VMS boot flags |
| `boot_file` | file name | empty | yes | File to load when the boot needs a name |
| `boot_reset` | `on`, `off` | `off` | yes | Do a full `init` before each boot |
| `console` | `serial` | `serial` | yes | Console device; only serial in vaxpunk |
| `os_type` | `vms` | `vms` | yes | Kept for compatibility; one OS today |
| `password` | hidden | unset | yes | Console password (15 to 30 characters) |
| `language`, `oem_string` | as real SRM | — | yes | Cosmetic; banner text |
| user variables | any | — | only if set in `nvram` | Created with `set name value` |

NVRAM holds the persistent variables, the password, the `nvram` power-up
script and a short event log. In vaxpunk it is a small file on the host,
presented to the guest as a flash device. Its format is vaxpunk's own and
versioned.

VMS boot flags are bits in the second number of `boot_osflags`, as in the
[OpenVMS Alpha installation manual](https://space.physics.uiowa.edu/pi/docs/vms83/opsys/vmsos831h1/ba322_90045/apas02.html):
1 conversational boot, 2 map XDELTA, 4 stop at the initial breakpoint,
8 diagnostic bootstrap, 10 stop at bootstrap breakpoints, 80 prompt for
the secondary bootstrap file, 100 halt before the secondary bootstrap,
10000 verbose boot messages, 20000 user-oriented boot messages. The
console passes them through; vaxpunk's VMS decides which it honors.

## Device naming

Devices use SRM's two-part names: a short name the operator types
(`DKA100`) and a long name that encodes where the device sits
(`dka100.1.0.7.0`). `show device` prints both:

```
>>> show device
dka0.0.0.7.0       DKA0      VAXPUNK-DISK   0.1
dka100.1.0.7.0     DKA100    VAXPUNK-DISK   0.1
dqa0.0.0.8.0       DQA0      VAXPUNK-CD     0.1
ewa0.0.0.9.0       EWA0      52-54-00-12-34-56
pka0.7.0.7.0       PKA0      SCSI Bus ID 7
```

The long name reads, left to right
([DS10 console reference](https://metebalci.com/blog/alpha-and-tru64-in-2023/ds10-console-reference.pdf),
table 2-4):

| Part | Example | Meaning |
| --- | --- | --- |
| Driver id | `dk` | Two letters for the kind of device: `dk` disk, `dq` CD/IDE, `ew` Ethernet, `pk` disk controller port |
| Adapter letter | `a` | Which controller of that kind: a, b, c |
| Unit number | `100` | Unit on the controller; SCSI units are 100 × node id |
| Bus node | `1` | Node id on the bus |
| Channel | `0` | For multi-channel controllers |
| Logical slot | `7` | Slot of the controller on the bus |
| Hose | `0` | Which I/O bus |

In vaxpunk the parts map onto the virtual machine: each virtio disk
becomes a `dk` unit, the network device is `ewa0`, a read-only install
image is `dqa0`. The slot and hose numbers come from the order devices are
found, so the same QEMU command line always gives the same names. Short
names are what `bootdef_dev` stores and what VMS sees as its device names.

## Boot flow and handoff to VMS

The console loads a small primary bootstrap from the disk, builds a
parameter block describing the machine, and jumps; VMS takes it from
there. Real SRM printed each step, and vaxpunk prints the same ones. A
DS10 booting OpenVMS showed this sequence
([DS10 console reference](https://metebalci.com/blog/alpha-and-tru64-in-2023/ds10-console-reference.pdf),
example 2-9):

1. Resolve the device and flags from the command line or `bootdef_dev` /
   `boot_osflags`, and echo the full command, e.g.
   `(boot dka0.0.0.7.0 -flags 0)`.
2. If `boot_reset` is on, run `init` first.
3. Read block 0 of the device and check it is a valid boot block. On
   OpenVMS the boot block gives the size and location of the primary
   bootstrap, `APB.EXE`, which must be contiguous
   ([OpenVMS manual](https://space.physics.uiowa.edu/pi/docs/vms83/opsys/vmsos831h1/ba322_90045/apas02.html)).
4. Read the bootstrap's blocks into memory and report base, start and
   size.
5. Build the HWRPB (hardware restart parameter block) at a fixed low
   address.
6. Build the initial page tables the bootstrap expects.
7. Set the initial machine state: registers, the PAL, the boot CPU.
8. With `-halt`, stop here at `>>>` so the operator can inspect, then
   `continue`.
9. Jump to the bootstrap, passing the HWRPB address.

The HWRPB is the contract between console and OS. Its layout is public
through the BSD and Linux Alpha ports
([NetBSD rpb.h](https://src.rcs.uwaterloo.ca/raw/netbsd-src/sys/arch/alpha/include/rpb.h)).
The parts vaxpunk keeps, in its own ARM64 version:

- A header: magic string "HWRPB", version, size, page size, physical
  address width, system serial number.
- A memory descriptor table: which physical ranges are usable, which
  belong to the console.
- A console terminal block: the console device and its settings.
- A console routine block: entry points for console callbacks (next
  section).
- A per-CPU slot: state flags (boot in progress, restart capable, PAL
  loaded, halted by operator), a halt-request code, and the saved context
  used for restart.
- A restart routine address, filled in by VMS, which the console calls to
  restart the OS after a halt.
- The boot parameters: device, file, root and flags as typed, so VMS knows
  how it was booted.

vaxpunk does not copy Alpha's bit layout. It keeps the names, the
structure and the rules, and defines the fields for ARM64.

## Console shell, files and the power-up script

The shell is small but Unix-flavored: pipes, redirection, background jobs,
and a RAM file system. It is not a programming language; scripts are
lists of commands.

- **Pipes and redirection.** Real SRM had simple pipes and I/O redirection
  ([NetBSD alpha boot(8)](https://man.bsd.lv/NetBSD-8.0/alpha/boot.8)).
  Devices and files share one byte-stream model, so a file can be
  redirected into physical memory: the DIGITAL guide's manual update
  loads an image with `cat fat:fwupdate.exe/dva0 > pmem:900000`
  ([DIGITAL SRM guide](https://ente.limmat.ch/ftp/pub/alpha/digital_server_7000/doc/srm/srmcons.pdf)).
- **Background jobs.** A trailing `&` runs a command as a console process;
  `show_status` lists them and `kill_diags` (or `kill`) stops them.
  Diagnostics such as `memexer 2 &` ran this way.
- **RAM files.** A memory-resident file system holds scratch files,
  command output and the event log (`more el`).
- **The `nvram` script.** A file named `nvram` kept in NVRAM runs on every
  power-up
  ([DS10 console reference](https://metebalci.com/blog/alpha-and-tru64-in-2023/ds10-console-reference.pdf),
  section 2.4). Typing `nvram` runs it by hand.
- **The editor.** `edit <file>` is a numbered-line editor: `nn text` adds
  or replaces line nn, `nn` alone deletes it, plus `list`, `renumber`,
  `exit` (save) and `quit` (discard).

In vaxpunk the `nvram` script is where pre-boot setup goes, for example:

```
10 set ewa0_ipaddr 10.0.2.15
20 start_sidecar net
30 echo nvram script done
```

A bad script must never brick the machine. Real SRM's answer was the halt
assertion: holding Halt during power-up skipped the script and auto-boot
once (DS10 reference, section 1.13). vaxpunk offers the same through a
host-side flag on the launch script and through a key pressed in the
first seconds of power-up.

The commands in this example other than `set` and `echo` are
placeholders; which helper commands exist is an open question.

## Halt, crash and console callbacks

The console stays useful after VMS is running: VMS can call into it, and
the machine can fall back to it.

**Getting back to `>>>`.** On a DS10, the Halt button or Ctrl/P suspended
OpenVMS and returned to the console; from there `continue` resumed it and
`crash` restarted it to write a crash dump
([DS10 console reference](https://metebalci.com/blog/alpha-and-tru64-in-2023/ds10-console-reference.pdf),
sections 1.12 and 2.12). vaxpunk enters the console the same ways:

- Ctrl/P on the serial line (or a host-side "halt" command for scripts).
- VMS executes its halt operation, for example after `SHUTDOWN` without
  reboot.
- A fatal error VMS cannot handle (a double fault in the executive).

On entry the console prints a halt code and the PC, saves the CPU state in
the HWRPB slot, and then follows `auto_action`.

**After an error, `auto_action` decides.** HP recommended `restart` for
OpenVMS: on a crash the console restarts the OS so it writes a dump, and
then the system reboots
([OpenVMS manual](https://space.physics.uiowa.edu/pi/docs/vms83/opsys/vmsos831h1/ba322_90045/apas02.html)).
With `halt` the machine waits at `>>>`. The state diagram above shows the
full set.

**Callbacks.** VMS's early boot code and its crash-dump path need console
I/O before VMS has drivers. Alpha SRM exposed a fixed table of routines
through the HWRPB's console routine block; the Linux Alpha port lists them
([Linux callback\_srm.S](https://git.zx2c4.com/wireguard-linux/tree/arch/alpha/lib/callback_srm.S)).
vaxpunk keeps a small subset:

| Callback | Use in vaxpunk |
| --- | --- |
| `getc`, `puts` | Console terminal I/O for the bootstrap, SYSBOOT and crash messages |
| `open`, `close`, `read`, `write` | Block I/O on the boot device for the bootstrap and the crash dump |
| `getenv`, `setenv`, `save_env` | VMS reads and changes console variables (e.g. from a DCL command) |
| `reset_term` | Put the console terminal back in a sane state |

Callbacks are only valid while VMS has not taken over the device;
afterwards VMS uses its own drivers.

## Security: secure mode and the console password

Secure mode exists for fidelity and for exposed demos; it is off by
default. The behavior follows the
[DS10 console reference](https://metebalci.com/blog/alpha-and-tru64-in-2023/ds10-console-reference.pdf),
sections 2.8 and 2.9:

- `set password` sets or changes it, 15 to 30 characters, never echoed;
  changing it asks for the old one.
- `set secure` turns secure mode on, but only if a password exists.
- In secure mode only `boot`, `login`, `continue` and `start` work, and
  `boot` takes no arguments: it uses the NVRAM defaults.
- `login` asks for the password and unlocks every command for this
  session. The console locks again on `boot`, `continue`, `start` or
  `init`.
- `clear password` needs the current password.
- A forgotten password is cleared by a halt during the `login` prompt. In
  vaxpunk that is a host-side action, since whoever controls the host owns
  the machine anyway.

The password is stored hashed in NVRAM; `show *` never prints it.

## Testing strategy

The boot tests (`boot/tests`) already drive the serial console and wait
for prompts; they drive `>>>` the same way. Each phase's "done when"
below is a boot test: auto-boot with no keys, Ctrl/C in the countdown
leaves `>>>`, a variable set survives a power cycle, `-flags 0,1` stops
in SYSBOOT, a halted VMS continues, a crash writes a dump and reboots.

## Open questions

- [ ] Implementation language: MACRO-32 like the executive, BLISS, or
  C/Rust for the first version?
- [ ] Which helper commands the `nvram` script needs to start sidecars,
  and what they are called. Real SRM had no such command.
- [ ] Exact console help and error message wording: copy real SRM's text,
  or write vaxpunk's own in the same style?
- [ ] `ps`, `kill`, `ls`, `echo`, `sleep`: assumed from memory of real SRM
  consoles, not confirmed in the manuals read for this PRD. Check a full
  AlphaServer console reference.
- [ ] Should `boot` accept a disk image file name directly
  (`boot -file x.img`) for development, or only devices?
- [ ] Event log: what goes in it, and how big it may grow in NVRAM.
- [ ] Does VMS ever write variables back (`setenv` + `save_env`), and from
  which DCL command?
- [ ] HWRPB versioning: how VMS rejects a console it doesn't understand.
- [ ] Should there be a remote console (SRM had Ethernet consoles), e.g.
  telnet to `>>>`, or is serial enough?

## Work order

The console grows in four phases, each usable on its own. The "Phase"
column in the command table refers to these.

1. **A prompt that boots.** Banner, `>>>`, line editing, `boot`, `show` /
   `set` / `clear`, `show device`, `show memory`, `show version`, `init`,
   `help`. Variables kept in a host file. `auto_action boot` with a few
   seconds to interrupt. Done when: QEMU starts, the console auto-boots
   VMS from `bootdef_dev`, and Ctrl/C during the countdown leaves you at
   `>>>`.
2. **A real handoff and a way back.** The HWRPB, boot block and primary
   bootstrap; `-flags` reaches VMS (conversational boot works); `-halt`
   and `continue`; Ctrl/P and VMS halts land at `>>>`; `examine` /
   `deposit`; `crash`; `auto_action restart`; the `nvram` script, `edit`,
   RAM files, `more`; the callback subset. Done when: a halted VMS can be
   inspected and continued, and a crash produces a dump and reboots.
3. **Operator comforts.** Secure mode and password, background jobs and
   `show_status`, `test` and `memtest`, event log, halt assertion. Done
   when: a scripted test run can drive the console end to end without
   timing hacks.
4. **The console owns the machine.** The PAL moves into the console, as on
   Alpha, and the console takes over machine-level duties currently done
   by the substrate. Done when: `show pal` reports vaxpunk's own PAL and
   VMS runs on it. Planned in a separate document.

## Sources

- [Alpha SRM Console for Alpha Microprocessor Motherboards, User's Guide](https://ente.limmat.ch/ftp/pub/alpha/digital_server_7000/doc/srm/srmcons.pdf) — DIGITAL, 1997
- [AlphaServer DS10 Console Reference](https://metebalci.com/blog/alpha-and-tru64-in-2023/ds10-console-reference.pdf) — Compaq, 1999
- [OpenVMS Alpha: Configuring Boot Behavior](https://space.physics.uiowa.edu/pi/docs/vms83/opsys/vmsos831h1/ba322_90045/apas02.html) — HP OpenVMS V8.3 Upgrade and Installation Manual
- [NetBSD alpha boot(8)](https://man.bsd.lv/NetBSD-8.0/alpha/boot.8)
- [NetBSD rpb.h (HWRPB layout)](https://src.rcs.uwaterloo.ca/raw/netbsd-src/sys/arch/alpha/include/rpb.h)
- [Linux callback\_srm.S (console callbacks)](https://git.zx2c4.com/wireguard-linux/tree/arch/alpha/lib/callback_srm.S)
- [OpenBSD alpha install notes (auto\_action)](https://src.rcs.uwaterloo.ca/source/raw/openbsd-src/distrib/notes/alpha/prep)
