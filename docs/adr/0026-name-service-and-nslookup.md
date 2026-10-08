# ADR-0026 — The resolver's configuration is TCP/IP Services' TCPIP$BIND_* logical names, which SET NAME_SERVICE sets, and the programs ask DNS themselves, through a resolver every image links

Oct 8, 2026 · @Marko Mikulicic

Proposed. `TCPIP SET NAME_SERVICE` and `SHOW NAME_SERVICE` keep the BIND
resolver's configuration in TCP/IP Services' logical names: the
process's, or with `/SYSTEM` the system's. `SYSTARTUP_VMS.COM` names
Google's public DNS server, `8.8.8.8`, for the system. A resolver
written in BLISS-64, `sysexe/lib/dns.b64`, is linked into every image.
It reads those names and asks the servers over a UDP socket.
`HOST_ADDR` asks it for any name the hosts database doesn't have, so
`PING`, `TELNET`, `SET HOST` and `COPY/HTTP` take DNS names. `nslookup`,
the foreign command `SYS$SYSTEM:TCPIP$NSLOOKUP.EXE`, is a BLISS-64
program on top of the same resolver. This covers the configuration
steps and the resolver of the
[research report](../research/openvms-dns.md)'s *What vaxpunk should
build first*, and its one tool, nslookup. It follows
[ADR-0025](0025-hosts-database.md).

## Context

[ADR-0025](0025-hosts-database.md) gave vaxpunk the hosts database, and
named DNS as a follow-up. There, each program looks names up itself,
through `HOST_ADDR` in `sysexe/lib/inet.mar`. On TCP/IP Services the
resolver's live configuration is a set of logical names:

- `TCPIP$BIND_SERVER000` to `002`, one server each;
- `TCPIP$BIND_DOMAIN`, and `TCPIP$BIND_DOMLST`, the `/PATH` search list;
- `TCPIP$BIND_RETRY`, `TCPIP$BIND_TIMEOUT`, `TCPIP$BIND_TRANSPORT` and
  `TCPIP$BIND_STATE`.

`SET NAME_SERVICE` writes the process's names, and with `/SYSTEM` the
system's. Because a resolver translates them as any program does, the
process table first, a process's settings hide the system's. `SET
CONFIGURATION NAME_SERVICE` keeps a permanent copy that startup loads.
The report recorded `SHOW NAME_SERVICE`'s exact layout and the V5.x
search rules. The binary encoding of the numeric logicals is not known.
On VMS, gethostbyname reaches the resolver through the network ACP.

The research recommended nslookup as the one BIND tool to build, since it
is the one VMS users remember. TCP/IP Services ships ISC BIND's nslookup,
in C. vaxpunk has no C compiler, but it has vbliss
([PRD-0004](../prd/0004-bliss64-compiler.md)), and UDP sockets with TCP/IP
Services' `$QIO` interface
([ADR-0024](0024-sockets-have-tcpip-services-qio-interface.md)).
vaxpunk's logical names hold one string each, which fits one server per
name.

No BLISS-64 program had run as an image on vaxpunk before. A P0 image
that called a system service from BLISS-64 failed to link: vbliss
compiles a call to `BL`, which reaches ±128 MB, and the services are in
S0, about 1 GB away. MACRO-32 reaches them because `G^` loads the address
first. BLISS-64 has no such operator, because on Alpha every call went
through a linkage pair. [DESIGN-0004](../design/0004-calling-standard.md)
already sets x16 and x17 aside for linker veneers, but vlink had none.

## Decision

1. **The configuration is the logical names, as text.** The names are
   TCP/IP Services', listed in *Context*. Without `/SYSTEM` they go in
   `LNM$PROCESS_TABLE`; with it they go in `LNM$SYSTEM_TABLE`, which
   needs SYSNAM. Readers translate each name through `LNM$FILE_DEV`.
   Numbers are stored as decimal text, `/PATH` as its domains joined by
   commas, and `/ENABLE` and `/DISABLE` as `ENABLED` and `DISABLED`.
   vaxpunk adds one name of its own, `TCPIP$BIND_PORT`, the servers'
   UDP port, 53 by default, so that a test can run a DNS server on the
   host without root.
2. **`SET NAME_SERVICE`** takes `/[NO]SERVER=(host,...)`,
   `/[NO]DOMAIN=domain`, `/[NO]PATH=(domain,...)`, `/RETRY=n`,
   `/TIMEOUT=n`, `/TRANSPORT=protocol`, `/ENABLE`, `/DISABLE` and
   `/SYSTEM`. A server or a `/PATH` domain goes after those already
   there, as on VMS. `/NOSERVER` and `/NOPATH` remove them all.
   **`SHOW NAME_SERVICE`** writes the report's layout: the local domain,
   then the system's settings and the process's. Where the system has
   no transport, retry or timeout, it shows the defaults: UDP, 2 and 5.
   Both are `tcpip.b64`, a BLISS-64 module that build.rs links into
   `TCPIP.EXE` with `tcpip.mar`. `TCPIP.CLD` calls its routines, and
   `TCPIP HELP` describes them, as it does every other command.
3. **There is no `SET CONFIGURATION NAME_SERVICE`.** What a boot
   should have goes in `SYSTARTUP_VMS.COM`, as with the hosts and the
   interface in [ADR-0025](0025-hosts-database.md). That procedure runs
   `TCPIP SET NAME_SERVICE /SERVER=8.8.8.8 /SYSTEM`.
4. **The resolver is `sysexe/lib/dns.b64`**, which build.rs links into
   every image, as it does `inet.mar`.
   - **Servers.** `DNS_SERVERS` takes them from `TCPIP$BIND_SERVER000`
     to `002`. A server given by name is looked up in the hosts database
     only (`HOST_LOCAL`), so a lookup never needs DNS to find DNS.
   - **Search.** `DNS_SEARCH` sends an A or a PTR query. For an A
     record, a name without a dot is tried in each `/PATH` domain, or
     else in the domain, and then as it is. NXDOMAIN, or an answer
     without the record, moves on to the next name.
   - **Timing.** Each server waits `TCPIP$BIND_TIMEOUT` seconds, 5 by
     default, and the whole list is tried `TCPIP$BIND_RETRY` times, 2 by
     default.
   - **No cache**, as on VMS.
   - **`DNS_ADDR`** returns a name's first A record, or
     `TCPIP$_NORECORD`.
5. **`HOST_ADDR` asks DNS second.** The hosts-database search is now
   `HOST_LOCAL`. `HOST_ADDR` calls it, and when the database hasn't the
   name, or there is no database yet, it calls `DNS_ADDR`, which is the
   order TCP/IP Services uses. Its callers are unchanged, and a name
   neither knows still gives `TCPIP$_NORECORD`.
6. **nslookup is `SYS$SYSTEM:TCPIP$NSLOOKUP.EXE`**, built from
   `sysexe/tcpip$nslookup.b64`, and `SYLOGIN.COM` defines it as the
   foreign command `NSLOOKUP`, as `TCPIP$DEFINE_COMMANDS.COM` does on
   VMS. Its syntax is `nslookup [-PORT=n] [-TIMEOUT=s] [-RETRY=n] [host
   [server]]`. With no host, it reads one per line after `>`, until
   `exit` or CTRL/Z.
   - **Lookups.** It asks the server the command names, or else the
     resolver's. A name gets an A query and an address a PTR query,
     through `DNS_SEARCH`.
   - **Output.** It writes BIND 9's text: `Server:` and `Address:`,
     `Non-authoritative answer:`, `Name:` and `Address:` lines,
     `canonical name =` and `name =`, `** server can't find name:
     NXDOMAIN`, and `;; connection timed out; no servers could be
     reached`.
7. **build.rs builds BLISS-64 programs and libraries.** It links
   `sysexe/NAME.b64` into `NAME.EXE`, together with `NAME.mar` if there
   is one. It links `sysexe/lib/*.b64` into every image, with
   `sysexe/lib/*.mar`.
8. **vlink adds range-extension veneers.** When a `B` or `BL` can't
   reach a fixed address, vlink notes the address and runs the link
   again with a veneer for it in a code psect of its own, `$VENEER$`.
   The veneer is `ldr x16, 8; br x16` followed by the address. A target
   that moves with the image keeps the out-of-range error, since a
   veneer's stored address would need a fixup.

## Alternatives considered

| Option | Why not |
| --- | --- |
| Turn on lwIP's DNS client and resolve in the TCP/IP component | Rejected in [ADR-0025](0025-hosts-database.md): it resolves below VMS, caches what TCP/IP Services doesn't, and doesn't know the hosts database or the logical names. |
| Use the server DHCP offers, or QEMU's `10.0.2.3` | DHCP's needs a new ioctl to reach VMS, and lwIP drops it with `LWIP_DNS 0`. A fixed public server works on any network that reaches the internet, which is all a vaxpunk needs. |
| `SET CONFIGURATION NAME_SERVICE` and `START COMMUNICATION` loading it | Its file is on the ramdisk, which is new at each boot. Saving it would only repeat what the startup procedure says. |
| Serve lookups from a process through `IO$_ACPCONTROL` on `BGA0:`, as INETACP does | It needs a process, and nothing calls gethostbyname or the ACP yet. The programs already look names up themselves (ADR-0025); the resolver goes where `HOST_ADDR` is. The ACP can call the same library later. |
| Keep the resolver inside nslookup | Then `PING` and `TELNET` couldn't use DNS. |
| Write it in MACRO-32, like the rest of `TCPIP` and `inet.mar` | Parsing DNS messages is the kind of work BLISS was made for, and the programs above the PAL are meant to be BLISS or MACRO (PRD-0004). |
| Port ISC BIND's nslookup | It is C, and it would bring BIND's license and size for what is a few hundred lines here. |
| `ADDRESSING_MODE(GENERAL)` on the services, or an `EXTERNAL` that vbliss always calls through a register | BLISS-64 code written for Alpha doesn't say it, and every call to a nearby routine would pay. A veneer costs only the calls that need it, and DESIGN-0004 planned for it. |

## Consequences

**What gets harder.**
- A name that is in neither the hosts database nor DNS now costs a DNS
  round trip before `%TCPIP-W-NORECORD`. Without a network, that is
  every server's timeout times the tries: 10 seconds by default.
- A network that intercepts DNS answers for `8.8.8.8` itself; vaxpunk
  sees what it says.
- Every image is larger by the resolver, about 2 KB of data and its
  code, whether it looks names up or not.
- `/NOSERVER=host` and `/NOPATH=domain` can't remove one entry: the
  command parser gives a negated qualifier no value. A fourth server
  is ignored without a message.
- `SHOW NAME_SERVICE` shows servers as they were given; VMS shows them
  by name, looked up in the hosts database. `SHOW HOST name` still shows
  the hosts database only; a name only DNS knows gives `NORECORD`
  there.
- The resolver sends A and PTR queries only, so there is no IPv6, and a
  truncated answer isn't asked again over TCP. `/TRANSPORT` and the
  state are kept and shown, but nothing reads them.
- DCL capitalizes a foreign command's words, so nslookup asks for
  `WWW.EXAMPLE.ORG` and shows the name in that case. DNS doesn't care,
  and VMS behaves the same. Quotes keep the case.

**What stays easy.**
- A program that takes a host still calls `HOST_ADDR`, and gets DNS with
  it. One that needs more than an address calls `DNS_SEARCH` and reads
  the answer as nslookup does.
- A new program can be written in BLISS-64, alone or next to its
  MACRO-32, and can call system services directly.

**Follow-ups:**
- `IO$_ACPCONTROL` on `BGA0:`, served by a process that calls
  `HOST_LOCAL` and then the resolver.
- `SHOW HOST name` asking DNS too, with its `BIND database` layout.
- `%TELNET-E-IVHOST` for a host neither knows.
