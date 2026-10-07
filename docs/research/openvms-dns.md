# OpenVMS resolves names through two resolvers, configured by logicals

On HP TCP/IP Services for OpenVMS (formerly UCX) V5.7, **there are two resolvers, and the classic gethostbyname doesn't resolve in the caller**. gethostbyname and gethostbyaddr in TCPIP$IPC_SHR issue an `IO$_ACPCONTROL` `$QIO` on a BG device. The network ACP, TCPIP$INETACP, answers it: it searches the local hosts database TCPIP$HOST.DAT, then asks DNS through the resolver in TCPIP$ACCESS_SHR (`TCPIP$RES_SEND`, `TCPIP$RES_SEARCH`, `TCPIP$RES_GETHOSTBYNAME`), which INETACP links against. getaddrinfo, getnameinfo and the res_* API instead run a port of the ISC BIND 9 resolver inside the calling process, also in TCPIP$IPC_SHR. Both read the same configuration. Experiments on the real V5.7-13ECO5F (see *Experiments on real OpenVMS*) back the split: with the stack stopped, gethostbyname fails with errno EBADF even for names in the hosts file and for dotted addresses, and `ANALYZE/IMAGE` shows INETACP linked against TCPIP$ACCESS_SHR. There is no client-side cache. The live configuration is a set of `TCPIP$BIND_*` logical names. A process-table definition (from `SET NAME_SERVICE`) overrides the system-table one (from `SET NAME_SERVICE /SYSTEM`). Startup loads the system values from the permanent record that `SET CONFIGURATION NAME_SERVICE` writes into TCPIP$CONFIGURATION.DAT. From V5.6 an optional Unix-style TCPIP$ETC:RESOLV.CONF replaces all of that for the BIND 9 resolver. For vaxpunk, the parts worth copying first are the configuration model (logicals plus TCPIP$CONFIGURATION), the hosts database with `SET/SHOW HOST` (whose exact record layout is now known), and the `SHOW NAME_SERVICE` / `SHOW HOST` displays and messages. None of these needs DNS on the wire. Since the real gethostbyname goes through the ACP `$QIO`, a vaxpunk resolver serving `IO$_ACPCONTROL` on `BGA0:` is the faithful design, not just a compatibility layer. Some details still conflict between sources or are inferred, and are flagged as such. The playground VM has no TCPIP-IP-CLIENT PAK, so the resolver, `SET/SHOW NAME_SERVICE` and the BIND logicals could not be exercised.

A note on sources. Citations marked *(kit)* name files in the TCP/IP Services PCSI kit on the OpenVMS Alpha V8.4-2L1 install CD (`VSI-AXPVMS-TCPIP-V0507-13ECO5F-1`), which is DCX-compressed and was expanded for this research; binutils' `bfd/vms-lib.c` documents the DCX format. Only short strings were quoted, and vaxpunk must reproduce behaviour and interfaces, never code from those images.

## Two resolvers: BIND 9 in the caller, and the ACP's for gethostbyname

getaddrinfo and the res_* API run in the caller. TCPIP$IPC_SHR in the V5.7-ECO5 kit is built from `[TCPIP_V57_BLECO5.SRC.BIND_RESOLVER]` (BIND 9 libbind: GETADDRINFO.C, RES_INIT.C, …). It carries the resolver's debug strings (`;; res_send()`, `;; res_query(%s, %d, %d)`) and its configuration search: first `tcpip$etc:resolv.conf`, honouring `LOCALDOMAIN` and `TCPIP$BIND_RES_OPTIONS`; failing that, `TCPIP$ACCESS_RES_INIT` reads the configuration database and the logicals (TCPIP$IPC_SHR.EXE *(kit)*). TCPIP$ACCESS_SHR exports `TCPIP$RES_*` entry points (INIT, QUERY, SEARCH, SEND, GETHOSTBYNAME, …). It contains the table of logical names `TCPIP$BIND_STATE`, `_DOMLST`, `_SERVER`, `_TRANSPORT`, `_DOMAIN`, `_RETRY`, `_TIMEOUT`, next to `LNM$SYSTEM_TABLE` and `LNM$PROCESS_TABLE` (TCPIP$ACCESS_SHR.EXE *(kit)*). The release notes agree from the outside. Turning on resolver debugging "can create an SSH packet corruption", so the debug output lands in the SSH server's own stream ([V5.6 RN 3.10.17](https://space.physics.uiowa.edu/pi/docs/vms83/network/tcpip56/tcprn/tcp_rnpro_003.html)). Fixes for threads calling getaddrinfo at the same time, and for memory leaks and "does not close the files properly", describe a per-process library ([V5.7 ECO2 RN](https://www.zx.net.nz/mirror/h30266.www3.hpe.com/odl/i64os/network/tcpip57eco2/tcprn/tcp_rnpro_005.html)). The network ACP, **TCPIP$INETACP.EXE, contains no `res_*`, `resolv` or `TCPIP$BIND` strings at all**. It only handles `TCPIP$INET_HOSTADDR` and an "IP cache" and "proxy cache" (TCPIP$INETACP.EXE *(kit)*). The manuals never say which path the C RTL takes; the experiments below show that gethostbyname and gethostbyaddr take the ACP path.

The `$QIO` path is fully documented as an interface. Assign a channel to `TCPIP$DEVICE:` (BG) and issue `IO$_ACPCONTROL`. The manual says it "accesses the network ACP to retrieve information from the host and the network database files". It searches the local hosts database, then BIND "if the BIND resolver is enabled" ([V5.6 Sockets, IO$_ACPCONTROL](https://space.physics.uiowa.edu/pi/docs/vms83/network/tcpip56/6529/6529pro_024.html)). The pieces don't fit together cleanly. **V5.7 ECO2 fix 4.13.33 says "QIO based hostname lookup takes longer than the intended 1 second when multiple pathnames or servers are configured on the bind resolver"** ([V5.7 RN](https://www.zx.net.nz/mirror/h30266.www3.hpe.com/odl/i64os/network/tcpip57eco2/tcprn/tcp_rnpro_007.html)), so the ACP path does consult BIND, with a short budget of its own. Fixes 4.13.21/22 describe INETACP deadlocks with "hundreds of outstanding requests" on its AQB work queue ([V5.7 RN](https://www.zx.net.nz/mirror/h30266.www3.hpe.com/odl/i64os/network/tcpip57eco2/tcprn/tcp_rnpro_006.html)). So INETACP is single-threaded, and a slow DNS answer inside it would hold up all ACP work. **Resolved by experiment:** `ANALYZE/IMAGE` of the installed TCPIP$INETACP.EXE lists four shareable images, TCPIP$ACCESS_SHR, LIBRTL, CMA$TIS_SHR and DECC$SHR, and not TCPIP$IPC_SHR. TCPIP$ACCESS_SHR carries a complete resolver (`TCPIP$RES_INIT`, `_MKQUERY`, `_SEND`, `_QUERY`, `_SEARCH`, `_QUERYDOMAIN`, `_GETHOSTBYNAME`, `_GETHOSTBYADDR`) and the logical names `TCPIP$BIND_DOMAIN`, `_DOMLST`, `_RETRY`, `_SERVER`, `_STATE`, `_TIMEOUT`, `_TRANSPORT`. So INETACP resolves through ACCESS_SHR, in its own process. TCPIP$IPC_SHR's gethostbyname, gethostbyaddr and gethostent carry iosb debug strings (`*gethostbyname st: 0x%x, iosb: 0x%x`), and they fail when the stack is down, so they reach INETACP by `$QIO`. That contradicts the in-caller reading of the release notes above: those fixes concern getaddrinfo and the BIND 9 res_* code, which really are in-process. MultiNet documents the same split explicitly. Its socket library resolves in the caller, and its UCX `$QIO` emulation is "referred to … the MULTINET_SERVER process, which then uses the DNS resolver routines" ([MultiNet 5.6 ch.7](https://process.com/docs/multinet5_6/install_admin/chapter_7.htm)).

UCX V4.2 already had the same model: "all computers use resolver code but not all computers run [a name server]". The optional name server ran as the separate process UCX$BIND_SERVER ([UCX V4.2 Mgmt ch.5](http://odl.sysworks.biz/disk$axpdocsep022/network/tcpip42/manage/6526pro_003.html)). It is TCPIP$BIND in V5.x, and the BIND server is not needed for a host to resolve names.

## Configuration: a permanent record, live logicals, and an opt-in RESOLV.CONF

The configuration exists at three levels, and the commands map onto them one for one. `SET CONFIGURATION NAME_SERVICE` writes the permanent record in TCPIP$CONFIGURATION.DAT, which "take[s] effect the next time the software starts up". `SET NAME_SERVICE /SYSTEM` changes the volatile, systemwide configuration. Plain `SET NAME_SERVICE` changes only the current process ([V5.6 Mgmt §6.9](https://space.physics.uiowa.edu/pi/docs/vms83/network/tcpip56/6526/6526pro_015.html)). Neil Rieck's captures show a `set name /server=yada` without /SYSTEM "went into PROCESS, not SYSTEM" ([Rieck](http://neilrieck.net/docs/openvms_notes_tcpip_services.html)). The UCX V4.2 manual lists the system-table form verbatim: `UCX$BIND_DOMAIN`, `UCX$BIND_SERVER000`…`002` (one address each), `UCX$BIND_RETRY`, `UCX$BIND_TIMEOUT`, `UCX$BIND_STATE`, and `UCX$BIND_TRANSPORT = "UDP"`. RETRY, TIMEOUT and STATE are shown only as dots, which suggests their values are binary ([UCX V4.2 App. A](http://odl.sysworks.biz/disk$axpdocsep022/network/tcpip42/manage/6526pro_017.html)). In V5.x the binaries name `TCPIP$BIND_*` with the same fields plus `TCPIP$BIND_DOMLST`, which is almost certainly the /PATH search list. The process table is looked at before the system table, and that is how a process setting overrides the systemwide servers ("the servers that are defined systemwide will not be queried", [VSI Command Reference p.124](https://docs.vmssoftware.com/docs/vsi-tcpip-services-for-openvms-management-command-reference.pdf)). Other stacks write these names too. MultiNet's DHCP client may change `UCX$BIND_SERVER00x` and `TCPIP$BIND_SERVER00x` ([MultiNet DHCP](https://www.process.com/docs/multinet5_6/install_admin/chapter_16.htm)), and its FAQ says to `DEFINE/SYSTEM/EXEC TCPIP$BIND_SERVER000 "127.0.0.1"` ([MultiNet DNS FAQ](https://process.com/support/multinet/faq/dns.html)). **Conflict:** the V5.7 binaries contain only the base string `TCPIP$BIND_SERVER`, with no `%03d`-style format near it. The numbered `000`…`002` form is therefore confirmed for UCX 4.2 and by third parties, but not for V5.7. The server list might instead be built in code, or be one search-list logical with several values.

The host's own identity is kept elsewhere. `SET [CONFIGURATION] COMMUNICATION /DOMAIN=` and `/LOCAL_HOST=` define `TCPIP$INET_DOMAIN`, `TCPIP$INET_HOST` and `TCPIP$INET_HOSTADDR`; gethostname returns host plus domain ([VSI Command Reference pp.82–84](https://docs.vmssoftware.com/docs/vsi-tcpip-services-for-openvms-management-command-reference.pdf)). TCPIP$INET_STARTUP.COM refuses to continue without a host name and domain unless an interface uses DHCP, and copies them to the `UCX$INET_*` names for old programs (TCPIP$INET_STARTUP.COM *(kit)*). There is no `SET DOMAIN` command. The resolver's own `Domain:` defaults to this local domain unless `/DOMAIN` is given.

RESOLV.CONF arrived with the BIND 9 resolver in V5.6. TCPIP$CONFIG extracts `TCPIP$ETC:RESOLV_CONF.TEMPLATE`, and renaming it to `RESOLV.CONF` turns it on. It then **"supersedes any configuration settings you implement with the TCP/IP management command interface … The two configuration methods cannot be used in combination."** Its directives are `domain`, `nameserver` (at most 3, IPv4 or IPv6), `search`, and `options` (`debug`, `ndots:N` default 1, `timeout:N` default 5, `attempts:N` default 2, `no-tld-query`, `edns0`, …). The logicals `LOCALDOMAIN` and `TCPIP$BIND_RES_OPTIONS` override the file ([V5.6 Mgmt §6.9](https://space.physics.uiowa.edu/pi/docs/vms83/network/tcpip56/6526/6526pro_015.html); shipped RESOLV.CONF *(kit)*). Two V5.6 documents disagree about IPv6. The IPv6 guide still says the resolver "has not yet been ported to communicate over IPv6", while the management guide and release notes say IPv6 transport works through RESOLV.CONF ([IPv6 guide](https://space.physics.uiowa.edu/pi/docs/vms83/network/tcpip56/6645/6645pro_004.html) vs [V5.6 RN](https://space.physics.uiowa.edu/pi/docs/vms83/network/tcpip56/tcprn/tcp_rnpro.html)). The IPv6 guide is probably stale.

DHCP is the weakest-documented part. The V5.6 client's CLIENT.PCY requests `dns_servers` and `dns_domain_name` by default ([V5.6 Mgmt ch.9](https://space.physics.uiowa.edu/pi/docs/vms83/network/tcpip56/6526/6526pro_027.html)). HP never says where the results go. **Inference, unverified for HP's client:** it updates the volatile system logicals rather than TCPIP$CONFIGURATION.DAT or RESOLV.CONF, which is what MultiNet's documentation says its own client does to the same names.

## Hosts first, then DNS, with search rules that changed at V5

Every HP source gives the same order: the hosts database first, and DNS only if the name is not there and the resolver is enabled. **`/DISABLE` sends "all name and address lookups … to the local hosts database"** (UCP help on the kit *(kit)*). From V5.6 the order is TCPIP$HOST.DAT, then TCPIP$ETC:IPNODES.DAT, then BIND. getaddrinfo walks all three databases for one record type before trying the next type ([V5.5 Sockets pp.4-20..4-23](https://space.physics.uiowa.edu/pi/docs/vms83/network/tcpip56/6529/ba548_90002.pdf)). TCPIP$HOST.DAT lives in `SYS$COMMON:[SYSEXE]` and is found through the `TCPIP$HOST` logical ([V5.6 Mgmt ch.1](https://space.physics.uiowa.edu/pi/docs/vms83/network/tcpip56/6526/6526pro.html)). It is an indexed RMS file, edited only through `SET [NO]HOST`, `CREATE HOST` (which seeds `LOCALHOST`, alias `localhost`, 127.0.0.1) and `CONVERT/VMS HOST` / `CONVERT/UNIX HOST` to and from /etc/hosts format ([V5.6 Command Reference](https://space.physics.uiowa.edu/pi/docs/vms83/network/tcpip56/6527/6527pro_003.html)). The plain-text `TCPIP$SYSTEM:HOSTS.DAT` that the kit still installs "is no longer used by the BIND resolver" ([V5.7 RN 4.2.12](https://www.zx.net.nz/mirror/h30266.www3.hpe.com/odl/i64os/network/tcpip57eco2/tcprn/tcp_rnpro_005.html)). Names are stored with their case and match in either case. DCL upcases a name unless it is quoted, so the convention is to add an all-upper or all-lower alias ([V5.6 Command Reference](https://space.physics.uiowa.edu/pi/docs/vms83/network/tcpip56/6527/6527pro.html)). Neither the record layout and keys of TCPIP$HOST.DAT nor the format of IPNODES.DAT is documented anywhere.

The search algorithm is the one real behavioural difference between versions. **UCX 4.x (BIND 4) appended the default domain, then removed its leftmost label repeatedly until two labels were left, then tried the bare name**: `owl.ucx.ern.sea.com`, `owl.ern.sea.com`, `owl.sea.com`, `owl`. V5.x (BIND 8/9) tries a dotless name with the default domain appended, then the bare name, unless a search list is set with `/PATH`. When TCPIP$CONFIG upgrades a UCX system, it builds a `/PATH` of the default domain and its parents (`ucx.ern.sea.com,ern.sea.com,sea.com`) to keep the old behaviour ([V5.6 Mgmt §6.9.5–6.9.6](https://space.physics.uiowa.edu/pi/docs/vms83/network/tcpip56/6526/6526pro_015.html)). The `SET NAME_SERVICE /PATH` help text describes the no-path case differently: try the name "as you typed it", then append the default domain. With BIND's `ndots:1` both descriptions hold, one for dotted names and the other for dotless ones. That reading is an inference. Repeated `/PATH` or `/SERVER` qualifiers append to the list, `/NOPATH` and `/NOSERVER` remove entries, and both lists hold at most three servers.

Retry and timeout are where the sources disagree most, and vaxpunk has to choose. The command reference and the V5.7-ECO5 help on the kit itself give **4 retries and 4 s**. They describe BIND 4-style doubling (4, 8, 16, 32 s, "Total = 1 minute for one server"), while SET CONFIGURATION's help gives a third formula, "timeout_value * retry_value * number_servers" ([VSI Command Reference pp.97–99, 121–124](https://docs.vmssoftware.com/docs/vsi-tcpip-services-for-openvms-management-command-reference.pdf); UCP help *(kit)*). The V5.6+ management guide gives **2 retries and 5 s**, with BIND 9's schedule: a 5 s timeout per server, then a second round at 10 s divided by the number of servers, rounded down, for **totals of 15, 20 and 24 s with 1, 2 and 3 servers** ([V5.6 Mgmt §6.9.3.2](https://space.physics.uiowa.edu/pi/docs/vms83/network/tcpip56/6526/6526pro_015.html)). The V5.6 SHOW samples and TCPIP$CONFIG's display after configuring also show 2/5, while V5.0-era samples show 4/4 ([DIGITAL V7.2-1 docs](https://ftp.zx.net.nz/rom/OVMSDOC_0721/721FINAL/6526/6526PROFILE_008.HTML)). The likely reading is that 4/4 was UCX's default and the help text was never updated. Nobody documents a resolver cache. Caching is the job of a local BIND server set up cache-only (`SET CONFIGURATION BIND /CACHE`), with the resolver pointed at 127.0.0.1 ([V5.6 Mgmt §6.8](https://space.physics.uiowa.edu/pi/docs/vms83/network/tcpip56/6526/6526pro_015.html)). INETACP's "IP cache" may cache `$QIO` lookups, but that is unconfirmed.

## Management commands and their exact output

All of these are `TCPIP>` commands, parsed by TCPIP$UCP. TCPIP.CLD defines only the DCL verbs `TCPIP` and `UCX`, each taking the rest of the line (UCP help *(kit)*). That is exactly the shape ADR-0022 gave vaxpunk's `TCPIP.EXE`. The syntax, from the VSI V5.7 command reference ([pp.97–124, 145–164](https://docs.vmssoftware.com/docs/vsi-tcpip-services-for-openvms-management-command-reference.pdf)), is:

```
SET NAME_SERVICE [ /CLUSTER=dev:[directory] ] [ /DISABLE ] [ /[NO]DOMAIN=domain ]
                 [ /ENABLE ] [ /INITIALIZE ] [ /[NO]PATH=domain ]
                 [ /RETRY=number_of_retries ] [ /[NO]SERVER=host ] [ /SYSTEM ]
                 [ /TIMEOUT=seconds ] [ /TRANSPORT=protocol ]
SET CONFIG [NO]NAME_SERVICE [ /[NO]SERVER=host ] [ /[NO]DOMAIN=domain ] [ /[NO]PATH=domain ]
                            [ /RETRY=n ] [ /TIMEOUT=seconds ] [ /TRANSPORT=protocol ]
SHOW NAME_SERVICE [ /STATISTICS ]
SET [NO]HOST host /ADDRESS=IP_address [ /[NO]ALIAS=alias ] [ /[NO]CONFIRM ]
SHOW HOST [ host ] [ /ADDRESS=IP_address ] [ /DOMAIN=domain ] [ /LOCAL ] [ /OUTPUT=file ] [ /SERVER=server ]
```

The privilege rules are as follows. /SYSTEM needs SYSPRV or BYPASS, plus SYSNAM. /PATH needs SYSNAM. /INITIALIZE and /STATISTICS need BYPASS, READALL or SYSPRV, and are BIND *server* operations: they run `MCR TCPIP$RNDC RELOAD` / `STATS`. /ENABLE and /DISABLE "must be used with /SYSTEM". /TRANSPORT takes UDP or TCP (the kit's help also lists SCTP). Values follow `=` or `:`, and lists go in parentheses. **Conflict:** the reference says "Do not use /NOSERVER with /SYSTEM", yet the management guide's own example is `SET NAME_SERVICE /NOSERVER=LARK /SYSTEM` ([VSI V6.0 Mgmt p.131](https://docs.vmssoftware.com/docs/vsi-tcpip-services-for-openvms-6-management.pdf)). The guide also uses `SET CONFIGURATION NAME_SERVICE … /ENABLE`, which the reference's format line leaves out. A clone should accept both.

`SHOW NAME_SERVICE` shows the volatile state. The HP V5.6 sample below is taken from the HTML `<pre>` block, so its spacing is the original ([HP V8.3 doc set](https://www.digiater.nl/openvms/doc/alpha-v8.3/83final/6526/6526pro_015.html)):

```
BIND Resolver Parameters

 Local domain: ucx.ern.sea.com

 System

  State:     Started, Enabled

  Transport: UDP
  Domain:    ucx.ern.sea.com
  Retry:     2
  Timeout:   5
  Servers:   lark
  Path:      ucx.ern.sea.com,ern.sea.com,sea.com

 Process

  State:     Enabled

  Transport:
  Domain:
  Retry:
  Timeout:
  Servers:
  Path:
```

A real system shows one detail the manuals don't. `SHOW CONFIGURATION` listed the servers as `67.69.184.87, 67.69.184.7`, but `SHOW NAME_SERVICE` printed `ns87_kawc99, NSR_DNS`, so **the live display reverse-maps server addresses through the hosts database** ([Rieck](http://neilrieck.net/docs/openvms_notes_tcpip_services.html)). UCP's strings give the full set of state words: `Started`/`Stopped`, `, Enabled`/`, Disabled`, `* Mismatch *`, and `No values defined` (TCPIP$UCP.EXE strings *(kit)*). No verbatim sample of a disabled resolver exists; `Started, Disabled` is a plausible guess. The permanent view prints its numbers in a different column from its strings ([HP V8.3](https://www.digiater.nl/openvms/doc/alpha-v8.3/83final/6526/6526pro_015.html)):

```
TCPIP> SHOW CONFIGURATION NAME_SERVICE

BIND Resolver Configuration

  Transport:  UDP
  Domain:     ucx.ern.sea.com
  Retry:         2
  Timeout:       5
  Servers:    9.20.208.47, 9.20.208.53
  Path:       No values defined
```

TCPIP$CONFIG reads this display back by line number: line 5 is Transport, then Domain, Retry, Timeout, Servers, and line 10 is Path (TCPIP$CONFIG.COM *(kit)*). The layout is therefore a de facto interface, and vaxpunk should match it line for line. `SHOW HOST` labels where an answer came from. For BIND it names the server that answered; for a wildcard it lists only the local database:

```
TCPIP> SHOW HOST ABCXYZ
      BIND database
Server:          128.182.4.164              ZSERVE
Host address              Host name
128.180.5.164             ABCXYZ.one.nam.com
```

```
     LOCAL database

Host address    Host name

127.0.0.1       LOCALHOST, localhost
67.69.184.7     NSR_DNS
207.164.234.128 yada.ca, YADA
```

The first sample is from the [VSI Command Reference p.155](https://docs.vmssoftware.com/docs/vsi-tcpip-services-for-openvms-management-command-reference.pdf), with alignment lost in the PDF. The second is a real capture from [Rieck](http://neilrieck.net/docs/openvms_notes_tcpip_services.html). With an IPv6 nameserver from RESOLV.CONF, the `Server:` line says only `IPv6`. A name that isn't found gives `%TCPIP-W-NORECORD, Information not found` / `-RMS-E-RNF, record not found` ([VSI V6.0 Mgmt p.325](https://docs.vmssoftware.com/docs/vsi-tcpip-services-for-openvms-6-management.pdf)). `SET NOHOST … /CONFIRM` shows the entry in the same LOCAL layout and asks `Remove? [N]:`.

TCPIP$CONFIG reaches the resolver through Core environment (main option 1), then `4 - BIND Resolver`; Domain is item 1 of the same menu. On a fresh system it says "A BIND resolver has not been configured." and explains that a server can be named or given as an address, but "if specified by name, an entry for it must exist in the TCPIP$HOST database". It then loops on `Enter your BIND server name:` / `Enter next BIND server name:` until an empty line. A name it doesn't know gets "odessy is not in the local host database." followed by `Enter Internet address for odessy:`, which leads to a `TCPIP SET HOST`. If no servers are given it prints "WARNING : No servers defined. The BIND resolver will not be enabled." On a configured system it shows the configuration and asks `* Do you want to reconfigure BIND [NO]:`. It never asks for retry, timeout or path; the answers are written with `SET CONFIG NONAME`, then `SET CONFIGURATION NAME /TRANSPORT=… /SERVER=(…) /PATH=(…)` ([VSI Install & Config p.28](https://docs.vmssoftware.com/docs/vsi-tcpip-services-for-openvms-installation-and-configuration.pdf); TCPIP$CONFIG.COM *(kit)*).

## Programming interfaces: the ACP function longword and the sockets calls

The `$QIO` interface is small and fixed:

| Argument | Meaning |
| --- | --- |
| P1 | Address of a descriptor of one longword: byte 0 = subfunction, byte 1 = call code, word 2 = MBZ |
| P2 | Descriptor of the input: a host name, or a dotted-decimal address |
| P3 | Address of a word that receives the output length |
| P4 | Descriptor of the output buffer |

The subfunctions (`$INETACPFSYMDEF`) are `INETACP_FUNC$C_GETHOSTBYNAME` = 1, `GETHOSTBYADDR` = 2, `GETNETBYNAME` = 3 and `GETNETBYADDR` = 4. The call codes (`$INETACPSYMDEF`) are `INETACP$C_ALIASES` = 1 (NUL-separated alias names; the length counts the NULs), `TRANS` = 2 (an address as 32 bits in network order), `HOSTENT` = 3, `NETENT` = 4, `HOSTENT_OFFSET` = 5 and `NETENT_OFFSET` = 6. The `_OFFSET` forms return a hostent or netent "with pointers replaced by offsets from the beginning of the structure". Codes 3 and 4 appear only in the header, not in the manual ([V5.5 Sockets pp.6-20..6-22, C-5](https://space.physics.uiowa.edu/pi/docs/vms83/network/tcpip56/6529/ba548_90002.pdf); [TCPIP$INETDEF.H in VSI CSWS](https://github.com/vmssoftware/csws)). NETLIB reads the offset hostent as five longwords — name offset, alias-list offset, addrtype, addrlen, addr-list offset — followed by data. The address list is an array of longword offsets ending in 0, each pointing at a 4-byte in_addr ([NETLIB](https://github.com/endlesssoftware/netlib)). HP's example program adds the buffer base to `h_name`, `h_addr_list` and `h_addr_list[0]`, and passes `&p4_dsc.dsc$w_length` as P3 so the returned length overwrites the descriptor's length ([V5.5 Sockets Ex. 2-25](https://space.physics.uiowa.edu/pi/docs/vms83/network/tcpip56/6529/ba548_90002.pdf)). The documented completion statuses are SS$_NORMAL, SS$_ABORT, SS$_BADPARAM, SS$_BUFFEROVF (aliases don't fit), SS$_ILLCNTRFUNC, SS$_NOPRIV, SS$_RESULTOVF, SS$_SHUT and, unusually, **SS$_ENDOFFILE for "The information requested is not in the database"**. Other BG functions add SS$_DEVNOTMOUNT ("INETACP is not currently available") and SS$_DEVINTACT. **Source conflict on GETHOSTBYADDR input:** HP's text says "All IP addresses are specified in dotted-decimal notation", and its example passes `inet_ntoa(addr)`. NETLIB, which ran against real UCX for years, passes the **4-byte binary address** with `TRANS` and reads back a name string. The likely reconciliation is that the ACP accepts both and tells them apart by P2's length (4 means binary), but that is unverified. It is also undocumented which status comes back when BIND doesn't answer, and what the IOSB's second word holds. One forum snippet that could not be fetched claims a HOSTENT_OFFSET buffer that is too small fails with SS$_ENDOFFILE rather than an overflow status.

The sockets side follows BSD. gethostbyname "searches the hosts database that is referenced by the TCPIP$HOST logical name … [and] may also invoke the BIND resolver" and returns a static hostent. The ACP call is named as its "$QIO equivalent" ([V5.6 Sockets ref](https://space.physics.uiowa.edu/pi/docs/vms83/network/tcpip56/6529/6529pro_014.html)). Failures set **h_errno**, which the C RTL has had since V7.0 through `decc$h_errno_get_addr()`. The netdb.h values on the CD are HOST_NOT_FOUND 1, TRY_AGAIN 2, NO_RECOVERY 3 and NO_DATA 4. The RTL's messages are "Unknown host", "Host name lookup failure", "Unknown server error", "No data record of requested type" and "No address associated with name" (DECC$RTLDEF.TLB / DECC$SHR.EXE *(kit)*). The CD's netdb.h settles the values that the online notes had left as unverified. Bad arguments set errno instead: `gethostbyname(0)` gives EINVAL with h_errno untouched, and ENETDOWN means TCP/IP isn't started ([V5.5 Sockets pp.3-26, 4-26..4-28](https://space.physics.uiowa.edu/pi/docs/vms83/network/tcpip56/6529/ba548_90002.pdf)). getaddrinfo/getnameinfo return `EAI_*` codes. They became thread-safe in V5.6, and their `NI_*`/`AI_*` flag values were changed by mistake and then restored in V5.6 ECO1/V5.7, so any vaxpunk headers should use the pre-V5.6 values ([V5.7 RN](https://www.zx.net.nz/mirror/h30266.www3.hpe.com/odl/i64os/network/tcpip57eco2/tcprn/tcp_rnpro_005.html)). Compiling with `_SOCKADDR_LEN` selects BSD 4.4 structures. A fixed bug made getaddrinfo skip the hosts database without it. The res_* API is shipped (`TCPIP$EXAMPLES:RESOLV.H`, 32-bit pointers only), and the header paths are `_PATH_RESCONF "tcpip$etc:resolv.conf"` and `_PATH_HOSTS "TCPIP$SYSTEM:HOSTS.DAT"` (RESOLV.H on the kit *(kit)*).

## Messages and utilities users actually see

A failed lookup appears as a primary message from the utility followed by a `TCPIP`-facility secondary giving the resolver's reason. TELNET to an unresolvable name printed `%TELNET-E-IVHOST, Invalid or unknown host localhost` / `-TCPIP-W-EAI_AGAIN, temporary failure` ([HPE forum, archived](https://web.archive.org/web/2020/https://community.hpe.com/t5/Operating-System-OpenVMS/telnet-localhost-error/m-p/5961789)). FTP printed `%TCPIP-E-FTP_NETERR, I/O error on network device` with the same secondary, and the Unix-style `ping` printed `ping: unknown host localhost.` ([HPE forum, archived](https://web.archive.org/web/2020/https://community.hpe.com/t5/operating-system-openvms/tcp-ip-host-problem/td-p/5135402)). A damaged hosts file gave `%TCPIP-E-HOSTERROR, cannot process host request` / `-TCPIP-W-NORECORD, information not found` / `-RMS-E-EOF, end of file detected` ([HPE forum, archived](https://web.archive.org/web/2020/https://community.hpe.com/t5/Operating-System-OpenVMS/Cant-Set-Host/td-p/5951323)). Dead servers give `%TCPIP-W-BIND_NOSERVNAM, Server with address 199.85.8.8 is not responding` and `%TCPIP-E-BIND_NOSERVERS, Default servers are not available` ([V5.6 Mgmt §6.12.3.1](https://space.physics.uiowa.edu/pi/docs/vms83/network/tcpip56/6526/ba548_90006.pdf)). The kit's TCPIP$MSG.EXE supplies the rest of the vocabulary (MSG strings *(kit)*):

- `GETHOST`: "invalid or unknown host !AS"
- `NOINETADDR`: "could not resolve address for host !AS"
- `DUPHOST`, `HOSTALIAS`
- `BINDENABLE`, `BINDDISABLE`
- `BINDNOTINIT`: "BIND resolver not initialized"
- `NOBINDOMAIN`, `NOBINDSERV`, `INSBINDDATA`
- `HOST_NOT_FOUND`: "no such host is known"
- `TRY_AGAIN`, `NO_RECOVERY`: "unexpected name server failure"
- `NO_ADDRESS`
- `EAI_AGAIN`/`FAIL`/`NONAME`: "unknown name or service"
- `EAI_FAMILY`, `EAI_MEMORY`, `EAI_SERVICE`, `EAI_SOCKTYPE`, `EAI_BADFLAGS`

Status `%X176481A2` is `CONFIGERROR` with severity E; TCPIP$CONFIG tests for it to mean "no name service configured". The facility is **0x764**, so codes are `0x0764xxxx`. Dumping the installed TCPIP$MSG.EXE with F$MESSAGE settles the names: the resolver group prints as `HOST_NOT_FOUND`, `NO_ADDRESS`, `NO_RECOVERY`, `TRY_AGAIN` (at 0x07649610–28), not `BIND_HOST_NOT_FOUND`, while the server-side ones do carry the prefix: `BIND_NO_INFORMA`, `BIND_NONAUTH`, `BIND_NOSERVERS`, `BIND_NOSERVNAM`, `BIND_READERR`, `BIND_NO_ZONEXFR`. Other confirmed codes are `GETHOST` (0x07648338), `NOINETADDR` (0x07648650), `IVHOST` (0x0764B890, "invalid or unknown host !AD"), `NAMEERROR` (0x07648530), `HOSTERROR` (0x07648358), `NORECORD` (0x076486A0), `NOCLILIC` (0x076485C0) and the `EAI_*` group from 0x07649630. The full list can be regenerated the same way (a DCL loop over F$MESSAGE from %X07648000 in steps of 8); the dump missed `EAI_NONAME` (about 0x07649658), which the dump's filter dropped. A message file doesn't store severities, so those still come only from what commands actually print: NAMEERROR and HOSTERROR print as E, NORECORD as W, INFOADDED and DUPHOSTNAME as I.

TCPIP$DEFINE_COMMANDS.COM defines the BIND 9 tools as foreign commands: `nslookup :== $sys$system:tcpip$nslookup.exe`, `dig`, `host`, plus `rndc` and the `bind_check*` tools. On the BIND 8 branch, `ndc` prints "ndc is obsolete; use rndc" (TCPIP$DEFINE_COMMANDS.COM *(kit)*). HP itself says "The nslookup utility is no longer recommended. Use the dig utility instead" ([V5.6 Mgmt §6.12.1](https://space.physics.uiowa.edu/pi/docs/vms83/network/tcpip56/6526/ba548_90006.pdf)). nslookup's VMS help says commands are recognised "only in all lowercase or UPPERCASE, not mIxEd case". Its error strings are `*** %s can't find %s: %s`, `*** Request to %s timed-out` and `*** Default servers are not available`. These are ports of ISC BIND, so their output should be stock BIND 9 text. No VMS transcript was found to confirm that.

The two Process Software stacks show which parts of this are VMS conventions and which are HP's choices. MultiNet's gethostbyname resolves in the caller, unlike HP's, which goes through its ACP. But **if DNS is enabled it consults its compiled host table only when DNS fails**. That table is installed as a global section, `MULTINET:NETWORK_DATABASE`. MultiNet also documents that its UCX `$QIO` emulation "does not perform domain-searching", and it keeps its configuration in `MULTINET_NAMESERVERS` and `MULTINET_SEARCHDOMAINS` (up to six, separated by blanks) ([MultiNet 5.6 ch.7](https://process.com/docs/multinet5_6/install_admin/chapter_7.htm); [messages](https://process.com/docs/multinet5_6/messages/chapter_2.htm)). TCPware uses `TCPWARE_NAMESERVERS` (up to 3), `TCPWARE_DOMAINLIST` (up to 6), `TCPWARE_RES_OPTIONS` (`ndots`) and `TCPWARE_RES_RETRIES`. It runs a detached resolver process (`@TCPWARE:STARTUP_RESOLVER DETACH`), defines `UCX$DEVICE` as `BG:`, and ships its own `UCX$IPC_SHR` ([TCPware 6.1 ch.3](https://process.com/docs/tcpware6_1/manage/chapter_3.htm); [TCPware 6.0 App. B](https://process.com/docs/tcpware6_0/html/users/appendix_b.htm)). **Conflict:** the TCPware FAQ gives the default `TCPWARE_SVCORDER` as `"local,bind"` ([FAQ](https://process.com/support/tcpware/faq/dns.html)), while the V6.0 logicals appendix gives `"bind,local"` ([App. B](https://process.com/docs/tcpware6_0/html/users/appendix_b.htm)). The default may have changed between versions. The point that matters for vaxpunk is that both vendors kept a BG device, the `UCX$`/`TCPIP$BIND_*` names and an ACP lookup path so UCX programs ran unchanged. Those names and that path are the compatibility surface. The lookup order is a choice each vendor made, and only HP's matters to vaxpunk.

## What vaxpunk should build first, and what can wait

vaxpunk's starting point fits well. ADR-0022 already gives it a `TCPIP` DCL verb whose image parses `TCPIP.CLD` with `CLI$DCL_PARSE`, which is how TCPIP$UCP works. It also has `SET CONFIGURATION INTERFACE` persisted to `DKB0:[000000]TCPIP$CONFIG.DAT`, `START COMMUNICATION` at boot, a `PING` command, and lwIP DHCP on QEMU's user network, whose DNS server is `10.0.2.3`. PRD-0008 added indexed RMS files and ANALYZE/RMS_FILE. The order below puts first what can be tested without UDP and leaves to last what needs a sockets library.

**First, with no network needed:**

1. The hosts database as an indexed RMS file located through a `TCPIP$HOST` logical, with the real record layout (fixed 271 bytes: a type byte, the dotted address in 15 bytes as key 1, the name in 255 bytes as key 0, both space-padded, duplicates allowed; see *Experiments*), `SET [NO]HOST /ADDRESS /ALIAS /CONFIRM`, `CREATE HOST` (seeded with LOCALHOST, alias localhost, 127.0.0.1) and `SHOW HOST [/LOCAL]` in the LOCAL layout above.
2. The `%TCPIP-W-NORECORD` / `-RMS-E-RNF` failure, which needs messages in facility 0x764.
3. The resolver configuration as system-table logicals (process table first): `TCPIP$BIND_DOMAIN`, `TCPIP$BIND_SERVER000`–`002`, `TCPIP$BIND_RETRY`, `TCPIP$BIND_TIMEOUT`, `TCPIP$BIND_TRANSPORT`, `TCPIP$BIND_STATE` and `TCPIP$BIND_DOMLST`. Store them as readable text, since the real binary encoding is unknown and nothing outside the resolver needs to parse it.
4. `SET NAME_SERVICE [/SYSTEM]` and `SHOW NAME_SERVICE` in the System/Process layout.
5. `SET/SHOW CONFIGURATION NAME_SERVICE` added to the existing configuration file. Its line order must match, because scripts parse it.
6. `START COMMUNICATION` loading the permanent record into the system logicals, and `TCPIP$INET_HOST`/`_DOMAIN` from `SET COMMUNICATION`.
7. When DHCP learns a server, it writes it to the volatile system logical only, as MultiNet documents (not confirmed for HP).

ADR-0022's file is named `TCPIP$CONFIG.DAT`, not `TCPIP$CONFIGURATION.DAT`. Whether to rename it can be decided in the new ADR.

**Next, once UDP and sockets exist:** a user-mode resolver library. This is vaxpunk's TCPIP$IPC_SHR plus ACCESS_SHR, with gethostbyname/gethostbyaddr and h_errno values 1–4. It searches the hosts database, then DNS. It reads the logicals, process before system. It applies the V5.x search rules: a dotless name gets the domain appended first, the name as typed is tried last, and `/PATH` replaces the domain list. It uses BIND 9 timing (2 tries, 5 s, second round 10 s divided by the number of servers) and keeps no cache. The default retry/timeout is the main decision to record in the ADR. 2/5 is what V5.6+ systems display and what the BIND 9 algorithm describes; 4/4 is what the help text says. The recommendation is **2/5 with BIND 9 timing**. The first consumers should be `TCPIP SHOW HOST name`, with the BIND layout and `Server:` line, and then `TCPIP PING` and TELNET by name. Those reproduce `%TELNET-E-IVHOST` with a `-TCPIP-W-EAI_AGAIN`-style secondary.

One shortcut needs a decision. lwIP has its own DNS client inside the component. Using it would mean resolution happens below the port rather than in a VMS process, and that client caches answers, which TCP/IP Services does not. (This comes from knowledge of lwIP, not from the research notes; check `lwipopts.h`.) Using it would make the ACP-style lookup on `BGA0:` cheap. But DNS behaviour would then be lwIP's rather than VMS's, and the hosts database, which needs RMS, would still have to be searched above it.

**Then, the ACP lookup, which real gethostbyname uses:** `IO$_ACPCONTROL` on `BGA0:` for subfunctions 1–4 with call codes 1, 2, 5 and 6. Return SS$_ENDOFFILE for "not found", SS$_BUFFEROVF/RESULTOVF for a buffer that is too small, SS$_ILLCNTRFUNC for unknown codes and SS$_BADPARAM for malformed input. Accept both dotted and 4-byte binary P2 for GETHOSTBYADDR. Serve it the way MultiNet does, with a helper process that calls the same library. Give it a short time budget, since real VMS intends about 1 s and a single queue that blocks on DNS is exactly the bug INETACP had.

**Defer:** TCPIP$CONFIG's BIND dialogue (unless vaxpunk grows a TCPIP$CONFIG), RESOLV.CONF and its exclusive-override semantics, IPNODES.DAT and IPv6, getaddrinfo/getnameinfo, TCP transport, `/INITIALIZE`, `/CLUSTER`, `/STATISTICS`, `SHOW HOST *` zone listings, the BIND server, `CONVERT/VMS HOST`, and dig/host/nslookup. If one tool is built, it should be a small `nslookup`, since it is the one VMS users remember. Defining the `UCX$` copies of the logicals is cheap and can come whenever an old program needs them.

## Experiments on real OpenVMS

Run on 7 Oct 2026 on the user's playground VM: OpenVMS Alpha V8.4-2L1 in AXPbox, with `VSI AXPVMS TCPIP V5.7-13ECO5F` installed but never configured and no TCP/IP license (`SHOW LICENSE` lists only BLISS32). `TCPIP SHOW VERSION` reports "HP TCP/IP Services for OpenVMS Alpha Version V5.7 - ECO 5". Running TCPIP$CONFIG created the six empty databases in `SYS$COMMON:[SYSEXE]` (TCPIP$SERVICE, HOST, NETWORK, ROUTE, PROXY, CONFIGURATION). Its Core environment menu marks Domain, Routing and BIND Resolver "No Client License" and refuses to open them. With `TCPIP$HOST` and `TCPIP$CONFIGURATION` defined `/SYSTEM/EXECUTIVE` by hand, as TCPIP$CONFIG does, the hosts commands work without a license.

**The hosts database.** `ANALYZE/RMS_FILE/FDL` and `DUMP/RECORD` of TCPIP$HOST.DAT show:

- Indexed, prolog 3, variable-format records that are always 271 bytes, with carriage-return carriage control.
- Byte 0 is the record type: 0 for a host's primary name, 1 for an alias. Every alias is its own record carrying the host's address.
- Bytes 1–15 are the address as dotted-decimal ASCII, padded with spaces. This is key 1 (string, 15 bytes at position 1, duplicates allowed, no compression).
- Bytes 16–270 are the name, case preserved, padded with spaces. This is key 0 (string, 255 bytes at position 16, duplicates allowed, key and record compression).
- The default file contains `LOCALHOST` (type 0) and `localhost` (type 1), both 127.0.0.1.

**Hosts commands, verbatim.** `SHOW HOST` output starts with a line of 7 blanks, then `     LOCAL database`, a line of 2 blanks, `Host address    Host name`, a line of 1 blank, then one line per host: the address left-justified in 16 columns and the primary name followed by `, alias` for each alias. `SHOW HOST/LOCAL` lists hosts in key-0 order of their primary names.

```
$ TCPIP SHOW HOST/LOCAL
       
     LOCAL database
  
Host address    Host name
 
10.1.2.3        FOO, FOOALIAS, bar
127.0.0.1       LOCALHOST, localhost
10.1.2.4        lower.example.org
```

- Names are matched without regard to case. `SHOW HOST BAR`, `SHOW HOST "Bar"` and `SHOW HOST LOWER.EXAMPLE.ORG` all find their lowercase entries, and an alias finds the whole host. There is no partial match: `SHOW HOST lower` fails.
- `SHOW HOST 10.1.2.3` (an address as the parameter) and `SHOW HOST/ADDRESS=10.1.2.4` both look up by address.
- A miss prints, with or without /LOCAL and with the resolver unlicensed:
  ```
  %TCPIP-E-HOSTERROR, cannot process host request
  -TCPIP-W-NORECORD, information not found
  -RMS-E-RNF, record not found
  ```
- Before `TCPIP$HOST` is defined, every hosts command prints `%TCPIP-E-HOSTERROR` / `-TCPIP-E-NOFILE, cannot access TCPIP$HOST database file` / `-RMS-E-FNF, file not found`. `SHOW HOST` prints the pair twice, the second time with an unformatted `!AS` in place of the file name.
- `SET HOST` on a name that already exists, with a different address, adds a second primary record and warns: `%TCPIP-I-INFOADDED, Host information added to database` / `-TCPIP-I-DUPHOSTNAME, duplicate TCPIP$HOST host name for FOO`.
- `SET NOHOST name` (with the default /CONFIRM) shows each matching host in the LOCAL layout and asks `Remove? [N]:`; anything other than Y or N asks again. `/NOCONFIRM` removes them all. Removing a primary name removes its aliases. Naming an alias fails with `%TCPIP-E-HOSTERROR, cannot process host request` / `-TCPIP-I-ALIAS, name is an alias for a host`.

**Name service without a license.** `SHOW NAME_SERVICE`, `SHOW CONFIGURATION NAME_SERVICE` and `SET NAME_SERVICE` all print `%TCPIP-E-NAMEERROR, error processing name service request` / `-TCPIP-E-NOCLILIC, TCPIP-IP-CLIENT PAK is not enabled`. `@SYS$STARTUP:TCPIP$STARTUP` prints `%TCPIP-E-STARTFAIL, failed to start TCP/IP Services` / `-TCPIP-E-NOLICENSE, license check failed`.

**gethostbyname with the stack down.** A MACRO-32 program that calls `DECC$CRTL_INIT` and then `DECC$GETHOSTBYNAME`, with `herror` and `perror` on failure, printed for `localhost`, `lower.example.org`, `LOWER.EXAMPLE.ORG`, `nope` and `10.1.2.4` alike:

```
gethostbyname: Error 0
gethostbyname: bad file number
```

So h_errno stays 0 and errno is EBADF. The library never read TCPIP$HOST.DAT itself, and didn't even parse the dotted address locally. It needs a channel to the stack first, which fits gethostbyname being the ACP `$QIO` described above.

**Images.** INETACP's shareable images are TCPIP$ACCESS_SHR, LIBRTL, CMA$TIS_SHR and DECC$SHR. TCPIP$IPC_SHR's are TCPIP$ACCESS_SHR, DECC$SHR, LIBRTL and CMA$TIS_SHR. The BIND tools are separate images: SYS$SYSTEM:TCPIP$NSLOOKUP.EXE, TCPIP$DIG.EXE and TCPIP$HOST.EXE.

**TCPIP$CONFIG.** It reads `SHOW CONFIGURATION NAME_SERVICE /OUTPUT=` back by line number (line 5 Transport, then Domain, Retry, Timeout, Servers, line 10 Path), and treats status `%X176481A2` (CONFIGERROR) as "nothing configured". It checks the logicals `TCPIP$BIND_ENABLE` and `TCPIP$BIND_STARTED` to show whether the BIND *server* is enabled and running.

**Left on the VM.** The six empty TCPIP$*.DAT files that TCPIP$CONFIG created (the hosts file holds only its default LOCALHOST entry again), and `DKA100:[DNS]` with the test procedures, the GHBN program and the message dump. The `TCPIP$HOST` and `TCPIP$CONFIGURATION` logicals were gone after the clean shutdown.

## Conflicts and open questions, and how to settle them

Some of these were settled on the playground VM on 7 Oct 2026 (TCP/IP V5.7-13ECO5F was already installed). The rest need TCP/IP running, which needs a TCPIP-IP-CLIENT PAK the VM lacks: `TCPIP$STARTUP` stops with `%TCPIP-E-STARTFAIL` / `-TCPIP-E-NOLICENSE`, and every name-service command with `%TCPIP-E-NAMEERROR` / `-TCPIP-E-NOCLILIC, TCPIP-IP-CLIENT PAK is not enabled`. As the user's notes require, check that no AXPbox is running on the playground disks (`pgrep`/`lsof`) before any disk write or boot.

| Issue | Status | How to settle |
| --- | --- | --- |
| Default retry/timeout: 4/4 (command reference, kit help) vs 2/5 (V5.6+ management guide, samples) | Conflict | `TCPIP SHOW NAME` and `SHOW CONFIG NAME` right after TCPIP$CONFIG on a fresh install; time a lookup against an unreachable server |
| `TCPIP$BIND_SERVER000`-style numbered logicals in V5.7 vs a single `TCPIP$BIND_SERVER` | Conflict (binary strings vs UCX 4.2 / MultiNet) | `SHOW LOGICAL/FULL TCPIP$BIND*` and `UCX$BIND*` after `SET NAME/SERVER=…/SYSTEM` and again after `SET NAME/SERVER=…` (process) |
| Access mode and encoding of RETRY/TIMEOUT/STATE values | Unknown | Same `SHOW LOGICAL/FULL`, then `DUMP` of the translation |
| GETHOSTBYADDR P2: dotted string (HP) vs 4-byte binary (NETLIB) | Conflict | A MACRO-32 program on the VM (MACRO-32 compiler is in the base OS) issuing both forms with TRANS and HOSTENT_OFFSET |
| ACP status when DNS is down; IOSB byte count; the too-small HOSTENT_OFFSET buffer | Unknown / forum snippet only | Same program with the server unreachable and a 16-byte buffer |
| Does INETACP call TCPIP$ACCESS_SHR for BIND, and does it cache? | **Settled:** INETACP links TCPIP$ACCESS_SHR, which has the full `TCPIP$RES_*` resolver; caching still unknown | Watch UDP traffic for a repeated `$QIO` lookup (needs the PAK) |
| Does gethostbyname use the ACP? | **Settled: yes.** With the stack down it fails with EBADF (h_errno 0) even for hosts-file names and dotted addresses; IPC_SHR's gethostbyname has `$QIO` iosb tracing | — |
| TCPIP$HOST.DAT keys and record layout; IPNODES.DAT format | **Settled** for TCPIP$HOST.DAT (see *Experiments*); IPNODES.DAT not checked | `TYPE TCPIP$ETC:IPNODES.DAT` |
| Printed message names and severities | **Names settled** (`HOST_NOT_FOUND`, not `BIND_HOST_NOT_FOUND`); severities only where seen | `TCPIP SHOW HOST nosuch.example` with BIND enabled (needs the PAK) |
| Dotless-name search order (as typed first vs domain first) | Two descriptions, plausibly ndots:1 | `SHOW HOST owl` and `SHOW HOST a.b` with resolver debug on |
| `/NOSERVER` with `/SYSTEM`; `/ENABLE` with SET CONFIG | Doc conflict | Type them on the VM |
| What HP's DHCP client does to the resolver logicals | Unverified (MultiNet analogy only) | `SET CONFIG INTERFACE WE0/DHCP` on QEMU user network, then `SHOW LOGICAL TCPIP$BIND*` |
| Disabled/stopped `SHOW NAME_SERVICE` output | No sample | `SET NAME/DISABLE/SYSTEM`, then `SHOW NAME` |
| TCPware SVCORDER default (`local,bind` vs `bind,local`) | Conflict | Doesn't matter to vaxpunk; leave it |
| VSI TCP/IP Services V6.x resolver on x86 | No source | Not needed for an Alpha-era clone |
| UCX 4.x qualifier differences, CMU-IP/WIN-TCP resolvers | Not researched / memory only | bitsavers UCX manuals if UCX fidelity ever matters |

## Conclusion

The central finding is that VMS put the name service in **logical names and libraries, with the network ACP as the classic entry point**. gethostbyname goes by `$QIO` to INETACP, which searches the hosts database and then DNS through TCPIP$ACCESS_SHR's resolver; getaddrinfo and res_* run BIND 9 in the caller. Both read the same logicals, and the "process overrides system" rule comes straight from the logical-name table search order. For vaxpunk that is good news. Most of what users and scripts see (`SET/SHOW NAME_SERVICE`, `SET/SHOW HOST`, TCPIP$CONFIGURATION, `START COMMUNICATION`, the messages) can be built and tested against the TCPIP utility, the logical-name services and RMS before a single UDP packet is sent, and the resolver can be added under them later without changing those commands.

The ACP `$QIO` is where copying VMS closely would also copy its weakness. A single ACP queue that blocks on DNS is exactly what produced INETACP's documented hangs. vaxpunk should keep the interface, since it is how real gethostbyname works, but serve it from a process that calls the shared resolver with a short time budget, so one slow DNS answer can't hold up other ACP work. The ADR should state that choice, along with 2/5 versus 4/4 and the decision on lwIP's caching DNS client, rather than leaving them implicit.
