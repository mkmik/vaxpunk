# PRD-0002 — TCP/IP through a port to an lwIP component

Oct 3, 2026 · @Marko Mikulicic

## Context and goal

vaxpunk gets TCP/IP by putting an off-the-shelf lwIP stack in a separate seL4 component and talking to it from the VMS executive through a DEC-style port. The executive sees an ordinary QIO network device. All the hard networking work (TCP state machine, virtio-net driver, buffers, interrupts) stays inside lwIP, so effort goes into the application layer instead.

Goals:

- Two vaxpunk systems can reach each other over TCP/IP using only native VMS interfaces ($QIO, IOSBs, ASTs).
- The networking stack below the port can be swapped for native MACRO/BLISS code later without touching the port driver or anything above it.
- Unblock the era-appropriate application layer: SET HOST-style remote login, VMS-to-VMS remote file access, mail and chat between office VMS boxes.
- Keep seL4 hidden: VMS-side code never sees a capability, endpoint or scheduling context ([ADR-0001](../adr/0001-pal-interface-vms-vocabulary.md)).

Decisions this PRD rests on:

| Decision | Reason |
| --- | --- |
| No DECnet (Phase IV or V) | Not interesting to rebuild; for DECnet archaeology, run a real copy of VMS |
| TCP/IP only | Historically fine: late VMS shipped TCP/IP Services alongside DECnet |
| lwIP as the bootstrap stack | C, small, very widely deployed, already used in the seL4 driver framework |
| Not gVisor netstack | Needs the Go runtime |
| Not smoltcp or Netstack3 for now | Considered; no advantage worth leaving the existing seL4 integration |
| Config owned by VMS | IP, mask and default gateway set from VMS, or by lwIP's DHCP client when VMS asks for it ([ADR-0022](../adr/0022-tcpip-utility-and-dhcp.md)) |
| QIO is the native interface | Sockets is only a userland library for ported applications |
| seL4 is scaffolding | End goal is fully native and self-hosted, in MACRO/BLISS |

## Non-goals

- Writing or tuning a TCP state machine, congestion control or a NIC driver.
- DNS resolver service, IPv6 configuration UI (lwIP may support them; not exposed yet).
- Emulating any real DEC hardware or reproducing MSCP/CI packet formats. Only the vocabulary and structure are borrowed.
- Performance work. The target protocols are low-bandwidth.

## Architecture

The VMS side and the lwIP component meet only at the port: shared frames plus a doorbell and a completion interrupt. Everything left of the port is permanent VMS code; everything right of it can be replaced later.

```
 VMS side (permanent)          │ port (shared frames)  │ lwIP component (replaceable)
                               │                       │
 applications ─> sockets lib   │                       │
   │              │            │                       │
   ▼              ▼            │                       │
 $QIO / $QIOW ─> port driver ──┼─> command ring ──────>┼─> adapter
 (TCPIP0)      │      ▲   <────┼── response ring <─────┼──   │
               │      │        │   credits, buffers    │   lwIP (raw API)
 MTPR #n,#PR$_DOORBELL│        │                       │     │
               ▼      │ device │                       │   virtio-net ──> NIC
              PAL ────┼────────┼── seL4_Signal ───────>┼─> notification
                      └────────┼── notification <──────┼── seL4_Signal
```

Applications reach the port driver either directly through $QIO or through the sockets library. The PAL and the component are the only parts that make seL4 system calls.

## Port model

The executive talks to the lwIP component through a port: a DEC smart-peripheral structure (in the spirit of SCA/MSCP) built from plain seL4 primitives. Nothing is emulated; every operation is a real seL4 call or a plain memory access.

| DEC term | seL4 mechanism | Notes |
| --- | --- | --- |
| Port | Shared frames mapped into both the executive and the component | Set up at boot; no runtime capability work |
| Command ring | Ring in the shared frames, written by the executive | Executive → component |
| Response ring | Ring in the shared frames, written by the component | Component → executive, completions can be out of order |
| Doorbell | `seL4_Signal` on the component's notification | One signal per batch of commands |
| Completion interrupt | Component signals the executive's notification; PAL delivers it as a device interrupt at a fixed vector and IPL | Only point where seL4 meets interrupt delivery |
| Connection | Logical channel ID carried in each message | One per TCP socket, plus a control connection |
| Credits | Counters in the shared frames | Each side posts only as many messages as the other has granted; bounds ring use and gives back-pressure |

Rules:

- The port is asynchronous end to end, like VMS QIO. The only synchronous seL4 operation in the path is the doorbell, which never blocks the caller.
- Data buffers (send and receive payloads) live in shared frames too, referenced by offset from ring entries. No bytes go through IPC message registers.
- Ring entries are fixed size. The exact layout is defined in the protocol section.

## PAL changes

The PAL gains two small things and keeps its rule: it speaks only VMS vocabulary, and all seL4 translation happens inside it. VMS-side code deals in port numbers, never capabilities.

1. **Doorbell as a processor register.** A new internal processor register, `PR$_DOORBELL`. The port driver writes `MTPR #port_index, #PR$_DOORBELL`. The MACRO-32 compiler turns `MTPR`/`MFPR` into PAL calls, as the Alpha MACRO-32 compiler did. The PAL maps the port index to the notification capability and issues `seL4_Signal`.
2. **Notification as interrupt.** At boot, each port's completion notification is bound to an interrupt vector in the system control block and an IPL. When the component signals it, the PAL delivers a normal device interrupt.

Implementation notes:

- Both are a few hand-written ARM64 instructions in the PAL (`svc #0`, syscall number in x7, capability in x0). No C shim and no libsel4 at run time.
- Syscall numbers come from the generated headers of the seL4 build (MCS configuration), not hardcoded.
- The port-index-to-capability table is built at boot from the system description. No runtime loading.
- Both go into [DESIGN-0001](../design/0001-pal-interface.md)'s tables when implemented.

## QIO interface and sockets library

Programs reach the network with $QIO on a network device, exactly like other VMS I/O. A port driver in the executive, written in MACRO-32 or BLISS, turns each QIO into port messages and turns completions back into IOSBs and ASTs.

Device model (proposed, to confirm): a template device `TCPIP0`. Assigning a channel to it and issuing the open function creates a per-connection unit, each with its own IOSB and AST context. This mirrors how TCP/IP Services for OpenVMS shaped its devices.

Function codes the driver needs at minimum:

| QIO function | Port message | Completes when |
| --- | --- | --- |
| Open socket (TCP or UDP) | `OPEN` | Component assigns a connection ID |
| Bind / listen | `BIND`, `LISTEN` | Component confirms |
| Accept | `ACCEPT` | A peer connects; a new unit is created |
| Connect | `CONNECT` | Handshake done or failed |
| Write | `SEND` | Data accepted by lwIP |
| Read | `RECV` | Data arrives, or peer closes |
| Close / deassign | `CLOSE` | Component releases the connection |
| Set/sense interface config | `IFCONFIG` | Control connection reply |

The driver stays in kernel mode and is not a system service. Error conditions map to standard `SS$_` codes in the IOSB.

**Sockets library.** A userland shareable image for ported software. It implements `socket`, `bind`, `listen`, `accept`, `connect`, `send`, `recv` and `close` as $QIOW calls on the device. It adds no kernel interface of its own. Started in the C run-time library ([ADR-0032](../adr/0032-c-run-time-library-and-ssl3-on-mbed-tls.md)): `socket`, `connect`, `send`, `recv`, `close`, `decc$socket_fd` and `gethostbyname`, linked into each program until shareable images can call each other; `bind`, `listen` and `accept` are left.

## lwIP component and port protocol

The lwIP component is a separate seL4 protection domain, built from the existing seL4 driver framework examples: virtio-net driver plus lwIP. It is the only new C code, and it lives below the port where C is acceptable.

Component behaviour:

- Active thread with its own MCS scheduling context, blocked on its notification. It wakes on the doorbell, on virtio interrupts and on lwIP timers.
- On wake: drain the command ring (within available credits), run lwIP, post responses, signal the executive once per batch.
- Uses the lwIP raw API internally; the socket-shaped protocol is implemented in a thin adapter inside the component.

Message protocol (socket-shaped, fixed-size ring entries):

| Field | Meaning |
| --- | --- |
| Type | `OPEN`, `BIND`, `LISTEN`, `ACCEPT`, `CONNECT`, `SEND`, `RECV`, `CLOSE`, `IFCONFIG`, `CREDIT` |
| Connection ID | Logical channel; 0 is the control connection |
| Request tag | Chosen by the port driver; echoed in the response so completions can arrive out of order |
| Status | Response only; mapped by the driver to an `SS$_` code |
| Buffer offset and length | Payload location in the shared data frames |
| Arguments | Small fixed fields: address, port, protocol, flags |

The protocol is versioned from day one (a version word in the shared frames), so the native replacement can speak the same messages.

## Configuration path

Network settings are owned by VMS. The component starts with no address and waits to be told.

1. A DCL command (name to decide, in the spirit of TCP/IP Services' `SET INTERFACE`) sets IP address, mask and default gateway, and stores them in a system configuration file.
2. The command issues a privileged set-config QIO on the network device.
3. The port driver sends an `IFCONFIG` message on the control connection.
4. The component applies it to lwIP and replies; the QIO completes.
5. At boot, a startup procedure replays the stored settings the same way.

VMS may tell it to ask a DHCP server instead ([ADR-0022](../adr/0022-tcpip-utility-and-dhcp.md)). Sense-config returns the current settings and link state for a `SHOW`-style command.

## Replace-later path

The port is the seam. Everything above it is permanent VMS code; everything below it is replaceable.

| Layer | Today | Native future |
| --- | --- | --- |
| Applications, sockets library, DCL commands | MACRO/BLISS/C on VMS | Unchanged |
| Port driver and QIO interface | MACRO-32 or BLISS | Unchanged |
| Rings, message protocol, credits | Shared seL4 frames | Plain memory, same layout and protocol version |
| Doorbell | PAL → `seL4_Signal` | PAL → direct wakeup of the native stack, or a real doorbell |
| Completion | Notification → PAL interrupt | Native interrupt or software interrupt |
| TCP/IP stack and NIC driver | lwIP + virtio-net in a seL4 component | Native MACRO/BLISS stack and driver |

Rule that keeps this true: no seL4 or lwIP idiom appears above the port. If the driver ever needs to know something about lwIP, it goes into the protocol as a message, not into the driver as a special case.

## Testing strategy

- Each step of the work order ends in something that runs in QEMU and is checked from the host: ping, a TCP connection to or from a host-side server, a console transcript.
- Port plumbing is tested on its own, before any networking: a test message round-trips through the rings and arrives as an interrupt at the expected vector and IPL.
- The two-system test runs two QEMU instances on a shared virtual network, scripted from the host.

## Open questions

- [x] Confirm the template-device model (`TCPIP0` cloning one unit per connection) versus one unit per socket opened explicitly. Template, named `BGA0:`, cloning `BGnn` units ([ADR-0016](../adr/0016-tcpip-component-and-bga0.md)).
- [x] Ring sizes, entry layout and data buffer pool size. Two rings of 32 messages of 32 bytes, 32 tags, 16 buffers of 4 KB ([DESIGN-0003](../design/0003-tcpip-port.md)).
- [x] Name and syntax of the DCL configuration commands. TCP/IP Services' own, in the `TCPIP` utility: `TCPIP SET INTERFACE WE0 /HOST=address /NETWORK_MASK=mask` or `/DHCP`, `SET ROUTE /DEFAULT /GATEWAY=address`, `SHOW INTERFACE`, and `SET CONFIGURATION INTERFACE` and `SET ROUTE /PERMANENT` to save them ([ADR-0022](../adr/0022-tcpip-utility-and-dhcp.md)).
- [x] UDP in step 4 or later. Later, with TCP/IP Services' $QIO interface for all sockets, UDP and raw ICMP ones too ([ADR-0024](../adr/0024-sockets-have-tcpip-services-qio-interface.md)).
- [ ] Which application-layer protocols to use: Telnet or something VMS-flavoured for SET HOST; what to use for remote file access, mail and chat. SET HOST is Telnet over TCP port 23, a character at a time, with the window's size (ADR-0027); the rest is open.

## Work order

1. **lwIP component boots.** seL4 driver framework example with virtio-net + lwIP runs in the vaxpunk QEMU image and answers ping from the host, with a hardcoded address.
2. **Port plumbing.** Shared frames, rings and credits set up at boot. `MTPR #n, #PR$_DOORBELL` reaches the component; a test message round-trips and arrives as an interrupt at the right vector and IPL.
3. **Control connection.** `IFCONFIG` sets address, mask and gateway from VMS; hardcoded address removed.
4. **TCP over QIO.** Port driver with `TCPIP0`: open, connect, send, receive, close. A MACRO-32 test program connects to a server on the host.
5. **Listen and accept.** A VMS program accepts a connection from the host.
6. **Two systems.** Two vaxpunk instances in QEMU talk to each other over a shared virtual network.
7. **Sockets library.** A small ported C program runs unchanged against it.
8. **First application.** SET HOST-style remote login between the two systems.
