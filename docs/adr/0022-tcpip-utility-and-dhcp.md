# ADR-0022 — The network is set with TCP/IP Services' TCPIP utility, which parses its own commands, and lwIP's DHCP client can give the interface its address

Oct 6, 2026 · @Marko Mikulicic

Proposed. `SET INTERFACE`, `SET ROUTE`, `SET CONFIGURATION INTERFACE`
and `SHOW INTERFACE` leave DCL's tables for `TCPIP.CLD`, which
`TCPIP.EXE` parses with `CLI$DCL_PARSE`, as TCP/IP Services' `TCPIP`
does: `$ TCPIP SET INTERFACE WE0 /HOST=address /NETWORK_MASK=mask`, or
`TCPIP` alone and the commands at its `TCPIP>` prompt. `/DHCP`, in place
of `/HOST` and `/NETWORK_MASK`, has the TCP/IP component's lwIP ask a
DHCP server for the address, mask and gateway. This replaces point 5 of
[ADR-0016](0016-tcpip-component-and-bga0.md).

## Context

ADR-0016 made the network's commands DCL verbs, with the address and the
mask as parameters, and kept DHCP out, as
[PRD-0002](../prd/0002-networking.md) did. On VMS, DCL has none of
them. They are TCP/IP Services' management commands: `TCPIP`, a DCL
verb, runs a utility that parses the rest of the line, or with none
prompts `TCPIP>`, against its own tables, as `MAIL` and `AUTHORIZE` do.
There an interface has a name, `WE0` for the first Ethernet, and takes
its address as `/HOST=` and its mask as `/NETWORK_MASK=`; `SET
CONFIGURATION INTERFACE /DHCP` has the DHCP client configure it at
startup, and `START COMMUNICATION` starts the network from the
configuration.

QEMU's user network has a DHCP server that gives the guest
`10.0.2.15/24` and the gateway `10.0.2.2`. lwIP has a DHCP client, which
runs on its timers; the component already runs them.
[ADR-0017](0017-command-tables-from-cld-with-vcdu.md) gave images
`CLI$DCL_PARSE` and `CLI$DISPATCH` with tables of their own.

## Decision

1. **`TCPIP` is a DCL verb**, whose image `TCPIP.EXE` takes the rest of
   the line with `LIB$GET_FOREIGN` and parses it with `TCPIP_TABLES`,
   from `sysexe/tcpip.cld`. Each command is a `ROUTINE`, which
   `CLI$DISPATCH` calls. With no command, it reads them after `TCPIP>`
   until `EXIT` or CTRL/Z, and reports each error with `LIB$SIGNAL`, as
   a warning, so that a severe one doesn't end it.
2. **The commands are TCP/IP Services'**: `SET INTERFACE WE0`, `SET
   CONFIGURATION INTERFACE WE0`, each with `/HOST` and `/NETWORK_MASK`
   or `/DHCP`; `SET ROUTE /DEFAULT /GATEWAY=address [/PERMANENT]`; `SHOW
   INTERFACE [WE0]`; `HELP [command]`, which describes them from the
   tables, as DCL's `HELP` does, with the same code; `START
   COMMUNICATION`, which `SYLOGIN.COM` runs at
   boot in place of `RUN TCPIP`. `WE0` is the one interface; another
   name is `SS$_NOSUCHDEV`.
3. **DHCP is lwIP's.** `IO$_SETCHAR` on `BGA0:` takes a fourth longword,
   flags, whose 4 asks for DHCP; the port's `IFCONFIG` has the flag
   `PORT_DHCP`. The component calls `dhcp_start` and answers once the
   server has given an address, or after 10 seconds with
   `PORT_ST_TIMEOUT`, `SS$_TIMEOUT`, while lwIP goes on asking. A static
   `IFCONFIG` stops DHCP first. ponytail: no ARP probe of the offered
   address (`LWIP_DHCP_DOES_ACD_CHECK 0`), which takes seconds.
4. **The saved configuration** is `INTERFACE address mask gateway`, then
   `DHCP` if `/DHCP` saved it, in `DKB0:[000000]TCPIP$CONFIG.DAT`.

## Alternatives considered

| Option | Why not |
| --- | --- |
| Keep the DCL verbs and add `/DHCP` to them | Quicker, but further from VMS each time a command is added; `TCPIP` is the name every TCP/IP Services manual and script uses. |
| `TCPIP :== $TCPIP`, a foreign command defined in `SYLOGIN.COM` | Works only after SYLOGIN, and not in a process that doesn't run it; TCP/IP Services adds `TCPIP` to DCL's tables. |
| A DHCP client in MACRO-32 above the port | Needs UDP messages on the port, broadcasts from `0.0.0.0` and lease timers in the executive's programs, for what lwIP has. On VMS the client is a process of its own because the stack has sockets; ours has no UDP on the port yet. |
| Answer `IFCONFIG` at once and have `TCPIP.EXE` poll for the address | A timer loop in MACRO-32 for each caller, where the component already waits for commands that take time. |
| Keep the ARP probe | Up to 8 seconds at every boot, on networks (QEMU's) where nobody else holds the address. |

## Consequences

**What gets harder.**
- Scripts and habits that used `SET INTERFACE address mask` must say
  `TCPIP SET INTERFACE WE0 /HOST=address /NETWORK_MASK=mask`.
- A boot whose configuration says DHCP, on a network without a server,
  waits 10 seconds, prints `%SYSTEM-W-TIMEOUT`, and goes on without an
  address until the server answers.
- `SHOW INTERFACE` doesn't say whether the address came from DHCP, and
  `SET ROUTE` on the running system stops DHCP there.

**What stays easy.**
- A new network command is a `ROUTINE` in `tcpip.cld` and `tcpip.mar`;
  DCL doesn't change.

**Follow-ups:**
- DNS, whose server QEMU's DHCP offers too (`10.0.2.3`), once there is a
  resolver.
- `SHOW INTERFACE /FULL` with the lease, and DHCP renewals in the
  console log.
