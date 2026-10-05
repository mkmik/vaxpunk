# DESIGN-0003 — TCP/IP through the port

Oct 3, 2026 · @Marko Mikulicic

How vaxpunk does TCP/IP: the TCP/IP component below the executive, the port
the two share, the executive's port driver, `BGA0:`, and the programs on
top of it. It implements [PRD-0002](../prd/0002-networking.md), with the
choices [ADR-0016](../adr/0016-tcpip-component-and-bga0.md) makes, and adds
to the PAL interface of [DESIGN-0001](0001-pal-interface.md): a doorbell
register, an interrupt and a field of the RPB.

```
 executive                       │ port (17 shared pages)      │ TCP/IP component
                                 │                             │
 TCPTEST, TELNETD, RTPAD, DCL    │                             │
   │ $QIO on BGA0: or a BGnn:    │                             │
   ▼                             │                             │
 NET$FDT ── command, tag ────────┼─> command ring ────────────>┼─> command()
   │  MTPR #0, #PR$_DOORBELL ──> PAL ── seL4_Signal ──────────>┼─> wakes
   │                             │                             │   lwIP, raw API
 NET$FORK <─ IPL 21, SCB ^X100 ──┼── PAL <── seL4_Signal ──────┼── respond()
   │  IOC$REQCOM                 │  response ring <────────────┼──   │
   ▼                             │  data buffers, by tag       │   virtio-net ──> NIC
 IOC$POST: IOSB, event flag, AST │                             │
```

## The TCP/IP component

`tcpip/` builds `tcpip.elf`, freestanding C: lwIP 2.2.1 (the `tcpip/lwip`
submodule) with its raw API and no OS (`NO_SYS`), configured by
`include/lwipopts.h` for IPv4, ARP, ICMP, TCP and UDP, a virtio-net driver
and the adapter between lwIP and the port (`src/main.c`). `include/` also
holds the few libc headers lwIP includes, and `src/libc.c` their
functions.

The root task embeds `tcpip.elf` (`roottask/src/tcpip.S`) and, if QEMU has
a virtio-net device, starts it (`start_tcpip` in `roottask/src/main.c`)
after `EXEC.EXE`. What it sets up is in `tcpip/include/component.h`:

| Component address | What |
| --- | --- |
| `0x200000`-`0x3FFFFF` | one 2 MB large page: the image, its data and stack, and its DMA buffers |
| `0x400000` | the port's 17 pages, the same frames as the executive's at `0x4FF00000` |
| `0x420000` | the page of virtio-mmio transports with the network device's |
| `0x421000` | the console UART, for its few lines |
| `0x422000` | its IPC buffer |

- **CSpace.** A CNode of 8 slots: 1, its notification, which it waits on;
  2, the PAL's notification, badged as the port's, which it signals; 3,
  the handler of the device's interrupt, SPI 16 + the transport's number,
  bound to its notification, which it acks.
- **Its notification.** The badge bits say why it woke: 1 the doorbell, 2
  the PAL's clock tick (every 10 ms, lwIP's timers), 4 the device.
- **Scheduling.** Priority 254, between the PAL's 255 and the executive's
  threads' 253, so the doorbell is answered before the executive goes on;
  a budget of 5 ms per 10 ms period.
- **Start.** x0 = the large page's physical address, from which it works
  out its DMA buffers', and x1 = its transport's offset in the page.

It starts lwIP's interface with no address, writes the port's version
and waits. Each time it wakes it takes received frames to lwIP, acks the
interrupt, runs lwIP's timers, carries out the commands in the command
ring, tries again the commands that wait, and signals the PAL once if it
wrote a response.

The device is modern virtio 1.x on virtio-mmio, as the disks are, with
VIRTIO_NET_F_MAC: 16 receive and 16 transmit buffers of 2 KB, a frame
each, copied in and out. lwIP's loopback (`LWIP_NETIF_LOOPBACK`) lets a
system connect to its own address.

## The PAL's part

- **The port's pages.** At boot, with a network device, the PAL makes 17
  pages of the executive's at `0x4FF00000` in S0, kernel write, and
  `RPB$L_PORT` holds that address; without one it is 0. The same frames
  are mapped in the component.
- **The doorbell.** `MTPR #n, #PR$_DOORBELL` (`PR$_DOORBELL` is 64; PAL
  call `MTPR_DOORBELL`, 0x48) signals port n's component; only port 0
  exists. It never blocks.
- **The completion interrupt.** The component signals the PAL's own
  notification with a badge of its own; the PAL requests an interrupt at
  IPL 21 and delivers it through SCB vector `^X100` as it does the
  console's. A fault of the component's stops the system with
  `%PAL-F-TCPIP`.

## The port

The executive's and the component's view of the same 17 pages,
`tcpip/include/port.h` and `$PORTDEF` in `vtools/lib/lib.mlb`, which match:

| Offset | What |
| --- | --- |
| 0 | `version`, `PORT_VERSION` (1) once the component is ready, else 0 |
| 4, 8 | `cmd_put`, `cmd_get`: commands written by the executive, taken by the component |
| 12, 16 | `rsp_put`, `rsp_get`: responses written by the component, taken by the executive |
| `0x100` | the command ring, 32 messages |
| `0x500` | the response ring, 32 messages |
| page 1 + n | tag n's data buffer, 4 KB, for tags 0-15 |

The indices run free; a message is at index mod 32. Each side writes
only its own ring and index. A message is 32 bytes:

| Offset | Field | |
| --- | --- | --- |
| 0 | type | `OPEN`, `BIND`, `LISTEN`, `ACCEPT`, `CONNECT`, `SEND`, `RECV`, `CLOSE`, `IFCONFIG`, `CANCEL` |
| 1 | flags | `IFCONFIG`: 1, set (not only sense); in the response, 2, the link is up |
| 2 | tag | the executive's, echoed in the response |
| 4 | connection | 0 is the control connection |
| 8 | status | the response's: OK, BADPARAM, NOMEM, INUSE, REFUSED, RESET, TIMEOUT, ABORTED, UNREACH, CLOSED |
| 12 | length | bytes in the tag's buffer |
| 16 | address | an IPv4 address, network order |
| 20 | port | a TCP port |
| 22 | protocol | `OPEN`'s: 6, TCP |
| 24, 28 | arg1, arg2 | `LISTEN`: the backlog; `ACCEPT`'s response: the new connection; `IFCONFIG`: the mask and the gateway |

| Command | Response when |
| --- | --- |
| `OPEN` | at once: the new connection, or NOMEM |
| `BIND`, `LISTEN` | at once |
| `ACCEPT` | a connection has come on the listening one: arg1 is its connection, address and port the peer's |
| `CONNECT` | the handshake is done, or REFUSED, TIMEOUT |
| `SEND` | lwIP has taken all of the buffer: length |
| `RECV` | data has come, at most length: length; or the peer has closed: CLOSED; or RESET |
| `CLOSE` | at once; the connection's waiting commands end with ABORTED |
| `CANCEL` | at once; the connection's waiting commands end with ABORTED |
| `IFCONFIG` | at once, with the address, mask, gateway and link state, after setting them if flagged |

Responses come in the order commands finish, not as they were sent. What
comes on a connection before a `RECV` waits in lwIP, which holds back its
window until the executive takes it.

**Credits.** A command takes a tag, which its response gives back. There
are 32 tags, as many as a ring holds, so neither ring can overflow, and
the executive never sends more than the component can hold. A `SEND` or
`RECV` needs one of the first 16, which have a buffer.

## The port driver

`roottask/exec/netdriver.mar`.

- `NET$INIT` finds the port in the RPB at boot; `EXEC$START` puts
  `NET$INTERRUPT` at SCB `^X100` and `NET$FORK` at software interrupt
  level 6, `IPL$_NETPOST`.
- `NET$FDT` makes an IRP for each `$QIO`, with the command after the
  packet and a send's data after that, copied from the caller's buffer at
  once. It sends the command if a tag is free, or queues the IRP on
  `BGA0:`'s UCB until one is: `SENDCMD` writes the command, copies a
  send's data to the tag's buffer, and rings the doorbell.
- `NET$INTERRUPT` requests `IPL$_NETPOST`; `NET$FORK`, at `IPL$_SYNCH`,
  completes each response's IRP with `IOC$REQCOM`, a receive's data copied
  into the IRP, where `IOC$POST` takes it to the caller's buffer, then
  sends what waited for a tag.
- **Statuses.** OK is `SS$_NORMAL`, BADPARAM `SS$_BADPARAM`, NOMEM
  `SS$_INSFMEM`, INUSE `SS$_DUPLNAM`, REFUSED `SS$_REJECT`, RESET
  `SS$_LINKABORT`, TIMEOUT `SS$_TIMEOUT`, ABORTED `SS$_ABORT`, UNREACH
  `SS$_UNREACHABLE`, CLOSED `SS$_LINKDISCON`. Without the component,
  `SS$_DEVOFFLINE`.
- **Cancel.** `IOC$CANCEL` calls `NET$CANCEL`. For `$CANCEL` the channel's
  requests on the port end with `SS$_ABORT` at once, and a `CANCEL` goes
  to their connection; for a channel that goes, at `$DASSGN` or image
  rundown, they are freed. Either way the tag stays taken, an orphan,
  until its response comes; an `OPEN` or `ACCEPT` that finds its request
  gone closes the connection it made.

### Devices

`BGA0:` is a template device, as TCP/IP Services' `BG0:` was. A channel
assigned to it opens a connection with `IO$_SETMODE` and goes to a unit
of its own, a UCB cloned from `BGA0:`'s, named `BGnn` (01-99), holding
the connection's number. `$ASSIGN` finds a unit by its name too, so a
process can take another's connection by name; the unit counts its
channels and closes the connection when the last goes.

| On | Function | Does |
| --- | --- | --- |
| `BGA0:` | `IO$_SETMODE` | opens a TCP connection: p1 = the socket's characteristics, a word protocol (6), a byte type and a byte family, as TCP/IP Services' |
| `BGA0:` | `IO$_SETCHAR` | sets the interface: p1 = its address, mask and gateway, 12 bytes |
| `BGA0:` | `IO$_SENSECHAR` | writes those to p1, then flags, 2 if the link is up: 16 bytes |
| unit | `IO$_SETMODE!IO$M_BIND` | binds to address p3, port p4 |
| unit | `IO$_SETMODE!IO$M_LISTEN` | listens, backlog p4 |
| unit | `IO$_ACCESS` | connects to address p3, port p4 |
| unit | `IO$_ACCESS!IO$M_ACCEPT` | accepts a connection on channel p4, assigned to `BGA0:`; the IOSB has the peer's port in its count and its address in the second longword |
| unit | `IO$_WRITEVBLK` | sends p2 bytes at p1, at most 4096 (`SS$_IVBUFLEN`) |
| unit | `IO$_READVBLK` | receives up to p2 bytes at p1, those that came; `SS$_LINKDISCON` once the peer has closed |
| unit | `IO$_DEACCESS` | closes the connection |
| unit | `IO$_SENSEMODE` | the IOSB's count is nn, the second longword the channels assigned |
| unit | `IO$_READPROMPT` | as a terminal: sends p5/p6, then receives a line, without its CR LF, with a carriage return as its terminator |
| unit | `IO$_SETMODE!IO$M_CTRLCAST`, `IO$M_CTRLYAST` | does nothing |

An address in p3 is a longword in network order: its first byte is the
first number. `IO$M_BIND`, `IO$M_LISTEN` and `IO$M_ACCEPT` are bits 10
and 11, past the terminal's modifiers, because a unit is a terminal too.

## Programs

- **`TCPIP.EXE`.** DCL's `SET INTERFACE address mask`, `SET ROUTE
  /DEFAULT /GATEWAY=address`, `SHOW INTERFACE` and `SET CONFIGURATION
  INTERFACE address mask` run it, which reads `OPTION`, `ADDRESS`,
  `MASK`, `GATEWAY` and `PERMANENT` from the parse
  ([ADR-0017](../adr/0017-command-tables-from-cld-with-vcdu.md)). As in
  TCP/IP Services, `SET INTERFACE` and `SET ROUTE` change the running
  system, and `SET CONFIGURATION INTERFACE` and `SET ROUTE /PERMANENT`
  the saved configuration, which the next boot applies. The first two
  sense the settings with `IO$_SENSECHAR`, change theirs and issue
  `IO$_SETCHAR`, so the gateway is still the interface's to the port and
  lwIP; the last two read the saved settings, change theirs and write
  `INTERFACE address mask gateway` in a new version of
  `DKB0:[000000]TCPIP$CONFIG.DAT`, the writable disk, where TCP/IP
  Services kept `TCPIP$CONFIGURATION.DAT` and `TCPIP$ROUTE.DAT` in
  `SYS$SYSTEM`; `SHOW INTERFACE` senses and prints. With no command, as `SYLOGIN.COM`
  runs it with `RUN`, it applies the saved configuration, unless the interface has
  an address already, prints `%TCPIP-I-SET`, and creates `TCPIP$TELNET`,
  the remote login server, unless it is there already. Without a network
  it does nothing. ponytail: one route, the default; a routing table
  when there is a second interface.
- **`TELNETD.EXE`**, process `TCPIP$TELNET`. Listens on TCP port 23; for
  each connection creates a process running `DCL.EXE` with the
  connection's unit, `_BGnn:`, as `SYS$INPUT`, `SYS$OUTPUT` and
  `SYS$ERROR`, named so, and keeps its own channel until the new DCL has
  assigned one. DCL quits when a read ends with `SS$_LINKDISCON` or
  `SS$_LINKABORT`; its last channel going closes the connection.
- **`RTPAD.EXE`**, DCL's `SET HOST address`. Connects to port 23 there,
  then waits for either of two reads, the connection's and the terminal's,
  with `$WFLOR`: what comes on the connection it writes on the terminal,
  each line typed it sends with CR LF. When the other end closes it prints
  `%REM-S-END`. Line at a time, edited locally; no Telnet options.
- **`TCPTEST.EXE`** tries both directions against the host
  (`image/tests/network.rs`).
- Programs print through `SYS$OUTPUT` (`lib/print.mar`), falling back to
  `OPA0:` for a process without one, so a remote session's output goes to
  its connection.

ponytail: `BGA0:` and its units aren't among the devices `$DEVICE_SCAN`
and `$GETDVI` see; a remote login has no username or password, and EDIT
still writes on the console.
