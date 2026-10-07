# ADR-0025 — Host names come from TCP/IP Services' hosts database, which the programs read themselves, and TCP/IP's files live on the ramdisk, which the startup procedure fills at each boot

Oct 7, 2026 · @Marko Mikulicic

Proposed. `TCPIP SET HOST`, `SET NOHOST` and `SHOW HOST` keep the hosts
database, an indexed file with TCP/IP Services' own record layout, and
`PING`, `TELNET`, `SET HOST` and `COPY/HTTP` take a host's name, an
alias or an address wherever they took an address. There is no DNS
yet. This is the first of the steps that the
[research report](../research/openvms-dns.md) lists under *What vaxpunk
should build first*. The hosts database and the saved network
configuration live on the ramdisk, `MDA0:`, which `SYSTARTUP_VMS.COM`
makes at every boot, and the data disk, `DKB0:`, is left for the user's
files. This replaces the file location in point 4 of
[ADR-0022](0022-tcpip-utility-and-dhcp.md).

## Context

Up to now every network command took only addresses, and lwIP's DNS
client is off (`LWIP_DNS 0`). On VMS with TCP/IP Services, names come
from two places: first the local hosts database, `TCPIP$HOST.DAT`, then
DNS through the BIND resolver, if one is enabled. The hosts database
needs no network and can be tested without UDP. The report settled its
layout on a real V5.7 system. It is an indexed file of 271-byte records:
a type byte (0 for a host's name, 1 for an alias), the address in
dotted text padded to 15 bytes (key 1), and the name padded to 255
bytes (key 0). Both keys allow duplicates. Names match in any case. The
report also recorded the exact `SHOW HOST` layout and the messages a
failed lookup prints, `%TCPIP-E-HOSTERROR` / `-TCPIP-W-NORECORD` /
`-RMS-E-RNF`.

Until now the saved network configuration, `TCPIP$CONFIG.DAT`, lived on
the data disk (ADR-0022), the one disk whose files outlive a boot. But
work on vaxpunk happens in many git branches, and each of them boots the
same `out/datadisk.img`. Settings saved while working in one branch then
show up in another, and a boot doesn't follow from the tree that built
it. The startup procedure is part of each branch's system disk, so
configuration it makes follows the branch.

On VMS, gethostbyname doesn't read the file itself. It sends an
`IO$_ACPCONTROL` `$QIO` to the network ACP, which reads the file and
then asks DNS. vaxpunk has no ACP process yet, and nothing in it calls
gethostbyname.

## Decision

1. **TCP/IP's files are on the ramdisk**, which `SYSTARTUP_VMS.COM`
   initializes and mounts at every boot, before anything else that can
   fail. The hosts database is `MDA0:[000000]TCPIP$HOST.DAT`,
   and the saved configuration that `SET CONFIGURATION INTERFACE` and
   `SET ROUTE /PERMANENT` write and `START COMMUNICATION` applies is
   `MDA0:[000000]TCPIP$CONFIG.DAT`. Both start empty at each boot. Hosts
   and a fixed address that a boot should have are `TCPIP` commands in
   `SYSTARTUP_VMS.COM`, which runs before `SYLOGIN.COM`'s `START
   COMMUNICATION`. With nothing saved, the interface asks DHCP, as
   before. Nothing at boot needs `DKB0:`; it is mounted last, if it has
   a volume.
2. **The hosts database has TCP/IP Services' record layout and keys.**
   Key 0 has key compression and key 1 has none, as on VMS. The first
   hosts command to find no file creates it, seeded with `LOCALHOST`,
   alias `localhost`, at `127.0.0.1`, as TCP/IP Services' `CREATE HOST`
   does.
3. **The commands are TCP/IP Services'**:
   `SET HOST host /ADDRESS=a [/ALIAS=(...)]`, `SET NOHOST host
   [/NOCONFIRM]` (which asks `Remove? [N]:` by default) and `SHOW HOST
   [host] [/ADDRESS=a] [/LOCAL]`. Their output and messages match the
   captures in the report.
4. **Programs look names up themselves.** `HOST_ADDR` in
   `sysexe/lib/inet.mar`, which is linked into every image, returns the
   address if the text is one, and otherwise reads the file and matches
   the name in any case. It returns `TCPIP$_NORECORD` for a name the
   file doesn't have. PING, RTPAD (`TELNET` and DCL's `SET HOST`) and
   COPY call it where they called `INET_ADDR`.
5. **TCP/IP Services' messages are in the executive's table**, facility
   0x764, with their real codes. `$PUTMSG` now writes a whole message
   vector: the first message with `%`, each following one with `-`, as
   VMS does.

## Alternatives considered

| Option | Why not |
| --- | --- |
| Serve lookups as the ACP does, `IO$_ACPCONTROL` on `BGA0:` | It is how gethostbyname works on VMS, but it needs a process to serve it, since the driver can't read an RMS file at device IPL. Nothing calls gethostbyname yet. When DNS comes, the ACP can call `HOST_ADDR` first. |
| Turn on lwIP's DNS client | It would resolve names below VMS, cache answers that TCP/IP Services doesn't cache, and still not know the hosts database. |
| Keep both files on the data disk, `DKB0:` | They outlive the tree that wrote them, and every branch shares them (see *Context*). |
| The system disk, where VMS keeps them (`SYS$COMMON:[SYSEXE]`) | vaxpunk's system disk is read-only, so `SET HOST` couldn't write it. |
| Build the files into the system disk image from the repo | The commands couldn't change them at run time, and the startup procedure already says the same thing in DCL. |
| A `TCPIP$HOST` logical name, as VMS has | vaxpunk's RMS translates logical names only in the device part of a file specification. |
| Our own text file in `/etc/hosts` format | Programs and procedures written for TCP/IP Services expect the indexed file, its keys and `SET HOST`. |
| Print the three-line errors from TCPIP itself | Every utility would need its own copy. VMS prints message vectors with `$PUTMSG`, and `LIB$SIGNAL` reaches it through the catch-all handler. |

## Consequences

**What gets harder.**
- Each lookup opens the file and reads every record, because names
  match in any case and key 0 doesn't. That is fine for tens of hosts.
- `SET HOST` and `SET CONFIGURATION INTERFACE` don't outlive a reboot.
  What should stay goes in `SYSTARTUP_VMS.COM`, which is the same for
  every vaxpunk built from one tree. Two machines on one network, as the
  network test boots, have to be given their addresses after they boot.
- The file gets the default protection, so a user outside SYSTEM's group
  can't read it. That doesn't matter until there are logins other than
  SYSTEM's.
- An alias belongs to the host at its address. Two hosts at one address
  share their aliases, and `SET NOHOST` removes both hosts' aliases.
- `SET HOST` on a name that is already there doesn't warn with
  `%TCPIP-I-DUPHOSTNAME`. `SET NOHOST` on an alias reports no record,
  where VMS says `-TCPIP-I-ALIAS`. The codes of those messages aren't
  known yet.

**What stays easy.**
- A new program that takes a host calls `HOST_ADDR`.
- A facility's message is a row in `getmsg.mar`, and a secondary message
  is another argument to `LIB$SIGNAL`.

**Follow-ups:**
- The resolver configuration (`SET/SHOW NAME_SERVICE`, the
  `TCPIP$BIND_*` logical names) and DNS over UDP, with the server that
  QEMU's DHCP offers.
- `IO$_ACPCONTROL` on `BGA0:`, served by a process that calls the
  hosts database and then DNS.
- `%TELNET-E-IVHOST` for an unknown host, which needs `!AD` in
  `$PUTMSG`.
