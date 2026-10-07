# ADR-0024 — Sockets have TCP/IP Services' $QIO interface, with UDP and raw ICMP, and the interface is set with its ioctls

Oct 7, 2026 · @Marko Mikulicic

Proposed. A program uses the network as it would on VMS with TCP/IP
Services: it assigns a channel to `TCPIP$DEVICE:`, makes a TCP, UDP or
raw ICMP socket with `IO$_SETMODE`, names sockets with `item_list_2`
and `item_list_3` descriptors of `SOCKADDRIN`s, and sets and senses the
interface and its route with ioctls, every symbol with TCPIP$INETDEF's
name and value. vaxpunk's own `IO$M_BIND` and `IO$M_LISTEN`, addresses
and ports in p3 and p4, the peer in the IOSB, and `IO$_SETCHAR`,
`IO$_SENSECHAR` and `IO$_ACCESS` on `BGA0:` for the interface and PING
are gone. This replaces the device interface of point 3 of
[ADR-0016](0016-tcpip-component-and-bga0.md);
[DESIGN-0003](../design/0003-tcpip-port.md) has the details.

## Context

ADR-0016's `BGA0:` took TCP/IP Services' `BG0:` as its model, but only
for its shape: a template device that clones a unit per socket, and the
socket characteristics in p1. The rest was ours. A bind and a listen
were modifiers that VMS doesn't have, `IO$M_BIND` and `IO$M_LISTEN`, on
the bits of `IO$M_OUTBAND` and `IO$M_PURGE`; `IO$M_ACCEPT` was `^X400`,
where VMS has `^X80`; an address and a port were p3 and p4 by value; an
accepted connection's peer came in the IOSB. On the template,
`IO$_SETCHAR` set the interface and `IO$_ACCESS` pinged, where TCP/IP
Services has `IO$_SETCHAR` the same function as `IO$_SETMODE`, which
makes a socket, and `IO$_ACCESS`, which connects one. A program written
for VMS would not run, and one written for vaxpunk would not run on VMS.
UDP had no port messages at all ([PRD-0003](../prd/0003-multi-user-vms.md)
tier 3, item 35).

The VSI TCP/IP Services Sockets API and System Services Programming
manual, chapters 5 and 6 and appendix B, gives the interface: the
function codes, what p1 to p6 are and how each is passed, the item
lists and the ioctls, and the condition values. The values of the
symbols are in TCP/IP Services' own `TCPIP$INETDEF.H` (V5.7) and UCX's
`UCX$INETDEF.H` (V2), which agree, and the `$IODEF` and `$SSDEF` bits
in dumps of Alpha V8.4's and VAX V7.3's STARLET. The nesting of an ioctl
list, an `item_list_2` of type `TCPIP$C_IOCTL` pointing at `ioctl_comm`
pairs, is the manual's and C-Kermit's and OpenPegasus's, which issue
them.

The port was already able to carry UDP: its message has a protocol, an
address and a port, which only `OPEN`, `BIND` and `CONNECT` used.

## Decision

1. **The interface is TCP/IP Services'.** On `BGA0:`, which the system
   logical name `TCPIP$DEVICE` names, as `TCPIP$STARTUP.COM` defines
   it (here `SYSTARTUP_VMS.COM`), `IO$_SETMODE` with p1 makes a socket:
   `TCPIP$C_TCP` and `TCPIP$C_STREAM`, `TCPIP$C_UDP` and
   `TCPIP$C_DGRAM`, or `TCPIP$C_ICMP` and `TCPIP$C_RAW`, in the family
   `TCPIP$C_AF_INET`. On the socket, `IO$_SETMODE` binds to p3, an
   `item_list_2` of type `TCPIP$C_SOCK_NAME`, listens with backlog p4,
   and takes ioctls in p5, in that order, in one request or several;
   `IO$_ACCESS` connects to p3, or for a datagram socket sets its peer;
   `IO$_ACCESS!IO$M_ACCEPT` accepts into the channel whose number is the
   word at p4, the peer's name to p3, an `item_list_3`;
   `IO$_WRITEVBLK` and `IO$_READVBLK` take a datagram's name in p3;
   `IO$_SENSEMODE` gives the local name in p3, the peer's in p4, ioctls
   in p6; `IO$_DEACCESS!IO$M_SHUTDOWN` shuts down the direction in p4.
   `IO$_SETCHAR` is `IO$_SETMODE` and `IO$_SENSECHAR` is
   `IO$_SENSEMODE`. The condition values are the manual's: a datagram
   with no peer and no name is `SS$_NOLINKS`, a name on a connected one
   `SS$_FILALRACC`, one too big `SS$_TOOMUCHDATA`, port 0
   `SS$_IVADDR`, another protocol or family `SS$_PROTOCOL`, a port below
   1024 or a raw socket without SYSPRV or BYPASS `SS$_NOPRIV`.
2. **The symbols are VMS's.** `starlet.mlb` gets `$INETSYMDEF`,
   `$SOCKADDRINDEF`, `$IFREQDEF`, `$ORTENTRYDEF` and `$SIOCDEF`, the
   TCPIP$INETDEF modules programs use, with their values;
   `IO$M_ACCEPT` is `^X80`, and `IO$M_SHUTDOWN`, `IO$M_EXTEND`,
   `IO$M_READATTN`, `IO$M_WRTATTN`, `IO$M_OUTBAND` and
   `IO$M_INTERRUPT` join it; the invented modifiers go.
3. **UDP and raw ICMP sockets in the component.** lwIP's UDP and raw
   PCBs; each socket keeps up to 8 datagrams with their senders, and a
   read takes one, the rest of it lost if it doesn't fit, as VMS's does.
   A raw ICMP socket gets a copy of each ICMP message, its IP header
   first, as BSD's do, and lwIP's ICMP still answers echo requests.
4. **The interface is set with ioctls on a socket.** `SIOCSIFADDR`,
   `SIOCSIFNETMASK`, `SIOCGIFADDR`, `SIOCGIFNETMASK` and `SIOCGIFFLAGS`
   on `WE0`; `SIOCADDRT`, `SIOCDELRT` and `SIOCGETRT` on the default
   route. `TCPIP.EXE` uses them, on a UDP socket, and pings on a raw
   ICMP socket, building the echo request and timing it itself, as
   ping does. DHCP has no ioctl on VMS, where TCP/IP Services runs a
   DHCP client process; here it is vaxpunk's own, `SIOCSIFDHCP`, in
   `$SIOCDEF`'s encoding.
5. **The port carries what the interface needs**, version 2: `OPEN`
   with an optional bind and listen, `SETMODE`, `GETNAME`,
   `SHUTDOWN`, `IFCONFIG` setting each field on its own, `SEND` with a
   destination and `RECV` with a sender; `PING` goes.
6. **What a request returns besides its IOSB**, a socket name or an
   ioctl's result, is written in the caller's process by a post routine
   the IRP names, `IRP$L_POST`, which `IOC$POST` calls before it writes
   the IOSB.

## Alternatives considered

- **Keep vaxpunk's interface and add UDP to it.** The smallest change,
  but more programs, TELNETD, RTPAD, TCPIP, TCPTEST, COPY/HTTP and
  whatever comes next, written against an interface no VMS has, and
  every C program ported later needing a shim. Fixing it now touches
  five programs.
- **The BSD sockets library first** (PRD-0002 step 7). On VMS the C
  library's sockets are built on this $QIO interface; they need it
  underneath, and can't hide a different one cheaply.
- **Interface settings through a device of their own**, off the socket
  device, so nothing collides with TCP/IP Services' codes. Less work
  than ioctls, but not how VMS does it; `TCPIP.EXE`, and any program
  reading an interface's address, would be vaxpunk's own.
- **PING kept in the component**, as a port message. lwIP does it
  easily, but a program can't do what VMS's ping does, and the raw
  socket it would need is the same work.
- **Cloning the unit at `$ASSIGN`**, as TCP/IP Services does. Needs a
  hook in `$ASSIGN` for one device; a program that makes its socket
  with `IO$_SETMODE` right after, as all do, sees no difference.

## Consequences

- Programs written to the manual, in MACRO-32 or later in C, run with
  their own symbols and descriptors; `TCPTEST` checks TCP both ways and
  UDP named and connected against the host.
- Unit numbers move: `TCPIP.EXE`'s UDP socket and PING's raw one take
  `BGnn` units too, so a remote login's process is `_BG03:` or later.
- The driver reads and writes item lists, and an IRP now carries where
  its results go; IOC$POST has a post routine call for that.
- Still missing, each marked in the code: buffer lists (p5, p6 of a
  write or read), socket options (`SS$_BADPARAM`), read and write flags
  such as `TCPIP$C_MSG_PEEK`, `IO$M_NOW`, out-of-band data, attention
  ASTs, IPv6, a routing table beyond the default route, a unit cloned
  at `$ASSIGN`, the UNIX error code in a failed read's or write's IOSB,
  and `BGnn`'s device class.
- Follow-ups: the sockets library (PRD-0002 step 7) on this interface;
  `IO$_ACPCONTROL` and a resolver for names; a TNA pseudo-terminal for
  remote logins, which would retire `IO$_READPROMPT` on a socket.
