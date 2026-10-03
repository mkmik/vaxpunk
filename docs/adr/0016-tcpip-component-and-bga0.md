# ADR-0016 — The TCP/IP component is lwIP and a virtio-net driver of our own that the PAL starts, and BGA0: clones a unit per connection

Oct 3, 2026 · @Marko Mikulicic

Proposed. The TCP/IP component of [PRD-0002](../prd/0002-networking.md)
is one freestanding ELF, `tcpip/`: lwIP's raw API, a small virtio-net
driver and the port adapter. The root task embeds it and starts it as a
thread in an address space of its own, as it starts the executive, not
as a Microkit protection domain from the seL4 Device Driver Framework.
The executive's device is `BGA0:`, a template: opening a connection
gives the channel a unit of its own, `BGnn`, which other processes can
assign by name. SET HOST is a line-mode remote login, `RTPAD.EXE` on
one side and `TELNETD.EXE` creating a DCL process on a network unit on
the other. [DESIGN-0003](../design/0003-tcpip-port.md) has the details.

## Context

PRD-0002 asks for lwIP in a seL4 component "built from the existing seL4
driver framework examples", behind a port, and leaves open: the device
model (`TCPIP0` cloning units, or a unit per socket), the ring sizes, the
DCL command, UDP, and the application protocols.

The seL4 Device Driver Framework (sDDF) and its lwIP examples are
Microkit systems: several protection domains (driver, virtualisers,
copiers, client, timer) wired by a system description that Microkit's
tool turns into a loader image. vaxpunk has no Microkit. Its root task is
the PAL and builds every object itself
([ADR-0002](0002-root-task-is-the-pal.md)), and the shim loads seL4 and
the root task, nothing else.

On the VMS side, devices are fixed UCBs in `qio.mar` with four-character
names, and processes write their output on the console's `OPA0:`.

## Decision

1. **One component, written here, started by the PAL.** lwIP 2.2.1 is a
   submodule, as seL4 is. The component is lwIP, a virtio-net driver for
   the transport the PAL finds, and the adapter, one thread polled from
   one notification. The PAL builds its CSpace, address space and
   scheduling context from the root task's untypeds, as it does the
   executive's, and gives it one 2 MB large page, so its DMA buffers'
   physical addresses follow from one.
2. **The port as PRD-0002 draws it.** 17 shared pages: a header, two rings
   of 32 messages of 32 bytes, and 16 buffers of 4 KB. Credits are 32
   tags, as many as a ring holds; a send or receive needs one of the 16
   with a buffer.
3. **`BGA0:` is a template device** (TCP/IP Services' `BG0:`), with four
   characters as the other devices: a channel opens a connection with
   `IO$_SETMODE` and goes to a cloned UCB, `BGnn`. Units count their
   channels, so a process can hand its connection to another by name.
4. **SET HOST is line mode over TCP port 23.** The remote side runs DCL in
   a new process whose `SYS$INPUT` and `SYS$OUTPUT` are the connection's
   unit, which reads lines and writes like a terminal without echo; the
   local side edits the line and sends it. Programs print through
   `SYS$OUTPUT`, not `OPA0:`.
5. **`SET INTERFACE address mask gateway`**, saved in
   `DKB0:[000000]TCPIP$CONFIG.DAT` and replayed by `SYLOGIN.COM`'s `RUN
   TCPIP`, and **`SHOW INTERFACE`**. TCP only for now; lwIP has UDP built
   in for when a port message needs it.

## Alternatives considered

- **sDDF's echo server as is.** Needs Microkit and its loader, or a
  re-implementation of Microkit's system description in the root task:
  more seL4 machinery than lwIP and a driver, for the same packets.
- **The virtio-net driver in the PAL**, as the disks' is, with lwIP above
  it. Puts C, lwIP and its timers in the PAL, the one task that must
  always answer the executive; and the PRD wants the stack swappable
  below a port.
- **`TCPIP0:`**, the PRD's proposal. Six characters don't fit the
  executive's DDCU device names; `BG` is the name TCP/IP Services used.
- **One unit per socket opened explicitly** (`$ASSIGN` to `BGnn` first).
  The program must then find a free unit; cloning on open needs no
  search, and VMS programs written for `BG0:` expect it.
- **A pseudo-terminal device (VMS's `RTA`, `TNA`) for SET HOST.** The
  right end state, with the terminal driver's line editing on the remote
  side and real Telnet; but the terminal driver serves only the console
  today. A network unit that reads lines does what DCL needs now.

## Consequences

- The PAL now builds two tasks besides itself; the executive's threads
  run one priority lower, below the component's.
- Changing the stack below the port touches only `tcpip/` and the PAL's
  `start_tcpip`; the protocol is versioned.
- A remote login has no username or password, and full-screen programs
  (EDIT's keypad mode) don't work over SET HOST until there is a
  pseudo-terminal.
- Follow-ups: the sockets library (PRD-0002 step 7), which needs a way to
  build C for vaxpunk; UDP messages; `$GETDVI` on network units; a
  terminal-class remote device.
