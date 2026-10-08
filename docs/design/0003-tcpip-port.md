# DESIGN-0003 — TCP/IP through the port

Oct 3, 2026 · @Marko Mikulicic

How vaxpunk does TCP/IP: the TCP/IP component below the executive, the port
the two share, the executive's port driver, `BGA0:`, and the programs on
top of it. It implements [PRD-0002](../prd/0002-networking.md), with the
choices [ADR-0016](../adr/0016-tcpip-component-and-bga0.md) makes, and adds
to the PAL interface of [DESIGN-0001](0001-pal-interface.md): a doorbell
register, an interrupt and a field of the RPB. Programs see TCP/IP
Services' $QIO interface ([ADR-0024](../adr/0024-sockets-have-tcpip-services-qio-interface.md)).

```
 executive                       │ port (17 shared pages)      │ TCP/IP component
                                 │                             │
 TCPTEST, TELNETD, RTPAD, TCPIP  │                             │
   │ $QIO on TCPIP$DEVICE:       │                             │
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

`tcpip/` builds `tcpip.elf`, freestanding C: lwIP 2.2.1 (the `pal/tcpip/lwip`
submodule) with its raw API and no OS (`NO_SYS`), configured by
`include/lwipopts.h` for IPv4, ARP, ICMP, TCP, UDP and raw ICMP sockets,
a virtio-net driver
and the adapter between lwIP and the port (`src/main.c`). `include/` also
holds the few libc headers lwIP includes, and `src/libc.c` their
functions.

The root task embeds `tcpip.elf` (`pal/src/tcpip.S`) and, if QEMU has
a virtio-net device, starts it (`start_tcpip` in `pal/src/main.c`)
after `EXEC.EXE`. What it sets up is in `pal/tcpip/include/component.h`:

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
`pal/tcpip/include/port.h` and `$PORTDEF` in `crosstools/vtools/lib/lib.mlb`, which match:

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
| 0 | type | `OPEN`, `SETMODE`, `ACCEPT`, `CONNECT`, `SEND`, `RECV`, `CLOSE`, `IFCONFIG`, `CANCEL`, `GETNAME`, `SHUTDOWN` |
| 1 | flags | `OPEN`, `SETMODE`: 1 bind, 2 listen; `SEND`: 1 to the address and port; `IFCONFIG`: 1 set the address, 2 the mask, 4 the gateway, 8 have DHCP set them; in its response, 16, the link is up |
| 2 | tag | the executive's, echoed in the response |
| 4 | connection | the socket; 0 for none |
| 8 | status | the response's: OK, BADPARAM, NOMEM, INUSE, REFUSED, RESET, TIMEOUT, ABORTED, UNREACH, CLOSED, NOLINKS, ISCONN, IVADDR |
| 12 | length | bytes in the tag's buffer |
| 16 | address | an IPv4 address, network order |
| 20 | port | a TCP or UDP port |
| 22 | protocol | `OPEN`'s: 6 TCP, 17 UDP, 1 a raw ICMP socket |
| 24, 28 | arg1, arg2 | `OPEN`, `SETMODE`: arg1 the backlog; `ACCEPT`'s response: arg1 the new connection; `IFCONFIG`: the mask and the gateway; `GETNAME`'s response: the peer's address and port; `SHUTDOWN`: arg1 0 receives, 1 sends, 2 both |

| Command | Response when |
| --- | --- |
| `OPEN` | at once: the new socket, bound and listening if flagged, or NOMEM, or what the bind failed with |
| `SETMODE` | at once: bound, listening |
| `ACCEPT` | a connection has come on the listening socket: arg1 is its connection, address and port the peer's |
| `CONNECT` | TCP: the handshake is done, or REFUSED, TIMEOUT; UDP, raw: at once, its peer set; ISCONN if it had one |
| `SEND` | TCP: lwIP has taken all of the buffer: length; UDP, raw: at once, one datagram sent, to the address and port if flagged, else to the peer; NOLINKS with neither, ISCONN with both |
| `RECV` | TCP: data has come, at most length: length; or the peer has closed: CLOSED; or RESET. UDP, raw: a datagram has come: as much of it as fits, the rest lost, its sender in address and port; a raw ICMP socket's has its IP header first |
| `CLOSE` | at once; the socket's waiting commands end with ABORTED |
| `CANCEL` | at once; the socket's waiting commands end with ABORTED |
| `IFCONFIG` | at once, with the address, mask, gateway and link state, after setting those flagged; with DHCP, once the server has given them, or after 10 seconds with `TIMEOUT`, while lwIP goes on asking |
| `GETNAME` | at once: the local address and port, and the peer's |
| `SHUTDOWN` | at once |

Responses come in the order commands finish, not as they were sent. What
comes on a TCP connection before a `RECV` waits in lwIP, which holds
back its window until the executive takes it; a UDP or raw socket keeps
8 datagrams, and drops more, as UDP may. A raw ICMP socket gets a copy
of each ICMP message, as BSD's do; lwIP's ICMP still answers echo
requests.

**Credits.** A command takes a tag, which its response gives back. There
are 32 tags, as many as a ring holds, so neither ring can overflow, and
the executive never sends more than the component can hold. A `SEND` or
`RECV` needs one of the first 16, which have a buffer.

## The port driver

`vms/exec/netdriver.mar`.

- `NET$INIT` finds the port in the RPB at boot; `EXEC$START` puts
  `NET$INTERRUPT` at SCB `^X100` and `NET$FORK` at software interrupt
  level 6, `IPL$_NETPOST`.
- `NET$FDT` makes an IRP for each `$QIO`, with the command after the
  packet, then where its socket names and ioctl results go, read from
  the caller's item lists, and a send's data last, copied from the
  caller's buffer at once. It sends the command if a tag is free, or queues the IRP on
  `BGA0:`'s UCB until one is: `SENDCMD` writes the command, copies a
  send's data to the tag's buffer, and rings the doorbell.
- `NET$INTERRUPT` requests `IPL$_NETPOST`; `NET$FORK`, at `IPL$_SYNCH`,
  completes each response's IRP with `IOC$REQCOM`, a receive's data copied
  into the IRP, where `IOC$POST` takes it to the caller's buffer, then
  sends what waited for a tag.
- **Results.** What a request returns besides its IOSB, a socket name
  or an ioctl's result, `NET$POST` writes from the response the IRP
  keeps: `IOC$POST` calls the IRP's post routine, `IRP$L_POST`, in the
  caller's process, before it writes the IOSB.
- **Statuses.** OK is `SS$_NORMAL`, BADPARAM `SS$_BADPARAM`, NOMEM
  `SS$_INSFMEM`, INUSE `SS$_DUPLNAM`, REFUSED `SS$_REJECT`, RESET
  `SS$_LINKABORT`, TIMEOUT `SS$_TIMEOUT`, ABORTED `SS$_ABORT`, UNREACH
  `SS$_UNREACHABLE`, CLOSED `SS$_LINKDISCON`, NOLINKS `SS$_NOLINKS`,
  ISCONN `SS$_FILALRACC`, IVADDR `SS$_IVADDR`. Without the component,
  `SS$_DEVOFFLINE`.
- **Cancel.** `IOC$CANCEL` calls `NET$CANCEL`. For `$CANCEL` the channel's
  requests on the port end with `SS$_ABORT` at once, and a `CANCEL` goes
  to their connection; for a channel that goes, at `$DASSGN` or image
  rundown, they are freed. Either way the tag stays taken, an orphan,
  until its response comes; an `OPEN` or `ACCEPT` that finds its request
  gone closes the connection it made.

### Devices

`BGA0:` is a template device, as TCP/IP Services' `BG0:` was, and the
system logical name `TCPIP$DEVICE`, which `SYSTARTUP_VMS.COM` defines,
names it. A channel assigned to it makes a socket with `IO$_SETMODE` and
goes to a unit of its own, a UCB cloned from `BGA0:`'s, named `BGnn`
(01-99), holding the socket's connection number and protocol. `$ASSIGN`
finds a unit by its name too, so a process can take another's
connection by name; the unit counts its channels, which `$GETDVI`'s
`DVI$_REFCNT` gives, and closes the connection when the last goes.

The functions are TCP/IP Services', with its symbols (`$INETSYMDEF`,
`$SOCKADDRINDEF`, `$IFREQDEF`, `$ORTENTRYDEF`, `$SIOCDEF` in
`starlet.mlb`). `IO$_SETCHAR` is `IO$_SETMODE` and `IO$_SENSECHAR` is
`IO$_SENSEMODE`. A socket name is an `item_list_2`, a word length, a word
`TCPIP$C_SOCK_NAME` and an address, to give one, or an `item_list_3`,
with the address of a longword for the length written too, to get one,
of a `SOCKADDRIN`: a word family, `TCPIP$C_AF_INET`, a word port and a
longword address, both in network order, and 8 zeros; with `IO$M_EXTEND`
one written is BSD 4.4's, a byte length and a byte family. An ioctl list
is an `item_list_2` of type `TCPIP$C_IOCTL` of `ioctl_comm`s, each a
longword request and the address of its argument.

| On | Function | Does |
| --- | --- | --- |
| `BGA0:` | `IO$_SETMODE` | makes a socket: p1 = its characteristics, a word protocol, a byte type and a byte family: `TCPIP$C_TCP` and `TCPIP$C_STREAM`, `TCPIP$C_UDP` and `TCPIP$C_DGRAM`, or `TCPIP$C_ICMP` and `TCPIP$C_RAW`, which needs SYSPRV or BYPASS; then p3 and p4 as on a unit |
| unit | `IO$_SETMODE` | binds to the name p3; a port below 1024 needs SYSPRV or BYPASS; listens, backlog p4; or the ioctls of p5, which need OPER: `SIOCSIFADDR`, `SIOCSIFNETMASK`, `SIOCSIFDHCP` (vaxpunk's) on `WE0`, `SIOCADDRT`, `SIOCDELRT` of the default route |
| unit | `IO$_SENSEMODE` | the local name to p3 and the peer's to p4; or the ioctls of p6: `SIOCGIFADDR`, `SIOCGIFNETMASK`, `SIOCGIFFLAGS` (`IFR$M_IFF_RUNNING` if the link is up) on `WE0`, `SIOCGETRT` of the default route, 4 at most |
| unit | `IO$_ACCESS` | connects to the name p3; a datagram socket's peer |
| unit | `IO$_ACCESS!IO$M_ACCEPT` | accepts a connection on a listening socket into the channel in the word at p4, assigned to `TCPIP$DEVICE:`; the peer's name to p3 |
| unit | `IO$_WRITEVBLK` | sends p2 bytes at p1, at most 4096 (`SS$_IVBUFLEN`, a datagram's `SS$_TOOMUCHDATA`); a datagram to the name p3 |
| unit | `IO$_READVBLK` | receives up to p2 bytes at p1, those that came, or one datagram, its sender's name to p3; `SS$_LINKDISCON` once the peer has closed |
| unit | `IO$_DEACCESS` | closes the socket; with `IO$M_SHUTDOWN`, p4 `TCPIP$C_DSC_RCV`, `_SND` or `_ALL` |
| unit | `IO$_READPROMPT` | vaxpunk's, as a terminal: sends p5/p6, then receives a line, without its CR LF, with a carriage return as its terminator |
| unit | `IO$_SETMODE!IO$M_CTRLCAST`, `IO$M_CTRLYAST`, `IO$M_READATTN`, `IO$M_WRTATTN`, `IO$M_OUTBAND` | does nothing; the ASTs never come |

ponytail: no buffer lists (p5, p6 of a write or read), socket options
(`SS$_BADPARAM`), read and write flags, `IO$M_NOW`, out-of-band
data, IPv6, or the UNIX error code in a failed read's IOSB; p3 and p4,
or the ioctls, in one request, not both; a unit is cloned at
`IO$_SETMODE`, not at `$ASSIGN`.

## Programs

- **`TCPIP.EXE`.** DCL's `TCPIP` verb runs it, and it parses the rest of
  the line, or with none each line after its `TCPIP>` prompt until
  `EXIT`, with its own tables, `sysexe/tcpip.cld`, and calls the
  command's `ROUTINE` with `CLI$DISPATCH`
  ([ADR-0022](../adr/0022-tcpip-utility-and-dhcp.md)). The commands are
  TCP/IP Services': `SET INTERFACE WE0` and `SET CONFIGURATION INTERFACE
  WE0`, with `/HOST=address /NETWORK_MASK=mask` or `/DHCP`, `SET ROUTE
  /DEFAULT /GATEWAY=address [/PERMANENT]`, `SHOW INTERFACE [WE0]`,
  `START COMMUNICATION`, `PING address [/NUMBER_PACKETS=n]`, which
  writes an ICMP echo request on a raw ICMP socket each second, its
  identifier the PID, its sequence number the count, its data when it
  went, and prints each reply that matches with its TTL and round trip,
  until CTRL/C or n of them and then how many came back, and `HELP`, which describes them from the
  tables with `HELP$TOPIC`, DCL's `HELP`'s code (`sysexe/help/`). `SET INTERFACE` and `SET ROUTE` change the
  running system, and `SET CONFIGURATION INTERFACE` and `SET ROUTE
  /PERMANENT` the saved configuration, which `START COMMUNICATION` applies. The
  first two sense the settings with ioctls on a UDP socket,
  `IO$_SENSEMODE` with `SIOCGIFADDR`, `SIOCGIFNETMASK`, `SIOCGIFFLAGS` and
  `SIOCGETRT`, change theirs and set them all with `IO$_SETMODE`,
  `SIOCSIFADDR`, `SIOCSIFNETMASK` and `SIOCADDRT`, or `SIOCSIFDHCP`; the
  last two read the saved settings, change theirs and
  write `INTERFACE address mask gateway`, and `DHCP` if `/DHCP` said so,
  in a new version of `MDA0:[000000]TCPIP$CONFIG.DAT`, on the ramdisk
  that `SYSTARTUP_VMS.COM` makes at each boot (ADR-0025; TCP/IP
  Services kept `TCPIP$CONFIGURATION.DAT` and `TCPIP$ROUTE.DAT` in
  `SYS$SYSTEM`); `SHOW INTERFACE` senses and prints.
  `START COMMUNICATION`, which `SYLOGIN.COM` runs, applies the saved
  configuration, or DHCP if none is saved, unless the interface has an address already, prints
  `%TCPIP-I-SET`, or the error, a DHCP server's timeout, and creates
  `TCPIP$TELNET`, the remote login server, unless it is there already.
  Without a network it does nothing. ponytail: one route, the default; a
  routing table when there is a second interface.
- **`TELNETD.EXE`**, process `TCPIP$TELNET`. Listens on TCP port 23; for
  each connection creates a process running `DCL.EXE` with the
  connection's unit, `_BGnn:`, which `$GETDVI`'s `DVI$_DEVNAM` gives, as
  `SYS$INPUT`, `SYS$OUTPUT` and `SYS$ERROR`, named so, and keeps its own
  channel until the new DCL has assigned one, `DVI$_REFCNT` 2. DCL quits when a read ends with `SS$_LINKDISCON` or
  `SS$_LINKABORT`; its last channel going closes the connection.
- **`RTPAD.EXE`**, DCL's `SET HOST address`. Connects to port 23 there,
  then waits for either of two reads, the connection's and the terminal's,
  with `$WFLOR`: what comes on the connection it writes on the terminal,
  each line typed it sends with CR LF. When the other end closes it prints
  `%REM-S-END`. Line at a time, edited locally; no Telnet options.
- **`COPY.EXE`**, `COPY/HTTP`: connects a TCP socket to the server and
  sends an HTTP/1.0 GET; the body goes into a STREAM_LF file.
- **`TCPTEST.EXE`** tries TCP both directions against the host, and UDP:
  a datagram from the host back to its sender, named, then on the socket
  connected to it (`boot/tests/network.rs`).
- Programs print through `SYS$OUTPUT` (`lib/print.mar`), falling back to
  `OPA0:` for a process without one, so a remote session's output goes to
  its connection.

ponytail: `BGA0:` and its units aren't among the devices `$DEVICE_SCAN`
sees, and `$GETDVI` sees a unit by its channel only; a remote login has
no username or password, and EDIT still writes on the console.
