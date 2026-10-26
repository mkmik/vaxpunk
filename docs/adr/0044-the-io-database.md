# ADR-0044 — The devices are VMS's I/O database: a DDB per controller on IOC$GL_DEVLIST, its units' UCBs by number, a dispatch table per driver, and units cloned from templates

Oct 25, 2026 · @Marko Mikulicic

Proposed. Every device is a UCB on one list, as on VMS: `IOC$GL_DEVLIST`
lists a device data block (DDB) for each controller, and each DDB lists
its units' UCBs in the order of their numbers. A device's name is its
DDB's name and its unit's number, `DKA0`, `TNA12`, `MBA3`, so names are
no longer four characters. Each driver has a driver dispatch table
(DDT) with its FDT, start I/O, cancel, unit initialization and cloned
UCB routines, which the executive calls through the UCB. A template
device, `BG0:`, `TNA0:` or `VTA0:`, is a unit that others are cloned
from, the first free number from 1 to 9999, and a clone is deleted when
its last channel goes. `$ASSIGN`, `$GETDVI` and `$DEVICE_SCAN` search
this one database, and `$GETDVI` answers from the UCB. This replaces
decision 1 of [ADR-0013](0013-qio-irps-and-drivers.md) (one UCB per
device and an FDT routine in it) and decision 1 of
[ADR-0019](0019-mailboxes.md) (mailboxes `MBnn` on a list of their own),
renames [ADR-0016](0016-tcpip-component-and-bga0.md)'s `BGA0:` to
`BG0:`, cloned at `$ASSIGN` as TCP/IP Services' is, and lifts
[ADR-0027](0027-terminals-are-ucbs-and-telnet-is-in-the-driver.md)'s
limit of 9 TELNET terminals.

## Context

[t8y:/exec/qio#io-database](../map/exec/qio.md): each driver kept its
own devices. `qio.mar` had an array of five UCBs (the disks, `BGA0:` and
`FTA0:`), `ttdriver.mar` a list of terminals, `netdriver.mar` a list of
network units and `mbdriver.mar` a list of mailboxes. `$ASSIGN` looked
through each in turn; `$DEVICE_SCAN` knew the disks, by their VCBs, and
the terminals, and nothing else, so `BGA0:`, its units and the
mailboxes were invisible to it and to `SHOW DEVICES`. A UCB's name was a
4-character field, which limited TELNET terminals to `TNA1` to `TNA9`,
mailboxes and network units to `nn`, and made the network's template
`BGA0:` where TCP/IP Services has `BG0:`. Unit numbers went round at 99
whether a unit had the number or not (PRD-0003 tier 2 #14). `IOC$CANCEL`
called each driver's cancel routine by name, and each driver freed its
own UCBs.

On VMS (the VAX/VMS internals books, and the V4.3 sources, read for the
design only) the I/O database is a list of DDBs, `IOC$GL_DEVLIST`, part
of the local system block. A DDB has the controller's name, `DKA`, a
pointer to its driver's DDT and to its first UCB; the UCBs are linked by
`UCB$L_LINK` in unit order. `IOC$SEARCHDEV` parses a name into the DDB's
name and a unit number: a name without its controller's letter is
controller A's (`DK0` is `DKA0`) and one without a number unit 0's.
`IOC$SCAN_IODB` walks every unit. A UCB has the device-independent
fields `$GETDVI` returns: `UCB$L_DEVCHAR`, `UCB$B_DEVCLASS`,
`UCB$B_DEVTYPE`, `UCB$W_DEVBUFSIZ`, `UCB$L_DEVDEPEND`, `UCB$W_UNIT`,
`UCB$W_REFC`, `UCB$L_STS`, `UCB$W_ERRCNT`; a terminal's
characteristics are these, the 12 bytes `IO$_SENSEMODE` returns. A
template has `UCB$V_TEMPLATE` set: `$ASSIGN` clones a unit from it
(`IOC$CLONE_UCB`), marks it `UCB$V_DELETEUCB`, calls the driver's
`DDT$L_CLONEDUCB`, and the last `$DASSGN` deletes it. `$DASSGN` and
`$CANCEL` call the driver's `DDT$L_CANCEL` for the channel.

Real OpenVMS Alpha V8.4-2L1 (AXPbox) shows what that looks like from
outside: `F$DEVICE` returns devices in the database's order, DDB by DDB,
units by number; `SHOW DEVICES` groups the disks, the terminals, then
the rest, and leaves out mailboxes and devices with `DEV$M_MBX` unless
named, `SHOW DEVICES MB`. `FTA0:` is a terminal, offline and
`/NOAVAILABLE`; `TNA0:` and `BG0:` are online templates, `UCB$L_STS`
`^X2010`. Mailboxes are `MBAn`, of class `DC$_MAILBOX`, type
`DT$_MBX`, with `REC`, `SHR`, `AVL`, `MBX`, `IDV` and `ODV`, their
buffer size their longest message; there is no `MBA0:`. `BG0:` has
class 0, `REC`, `NET`, `AVL`, `MBX`, `IDV` and `ODV`, buffer size 256.
TCP/IP Services' startup defines `TCPIP$DEVICE` as `_BG0:`.

## Decision

1. **`$DDBDEF`, `$DDTDEF` and `$UCBDEF` with VMS's names** in
   `lib.mlb`. A DDB: `DDB$L_LINK`, `DDB$L_UCB`, `DDB$W_SIZE`,
   `DDB$B_TYPE`, `DDB$L_DDT`, `DDB$T_NAME`. A DDT: `DDT$L_START`,
   `DDT$L_FDT`, `DDT$L_CANCEL`, `DDT$L_UNITINIT`, `DDT$L_CLONEDUCB`,
   `IOC$RETURN` for what a driver hasn't. A UCB starts with its queue of
   IRPs, then `UCB$W_SIZE` and `UCB$B_TYPE`, as every block of pool,
   then its DDB, link, DDT, VCB, owner `UCB$L_PID` and the
   device-independent fields above; each kind of device adds its own
   after `UCB$K_LENGTH`. The `DDB`, `UCB` and `DDT` macros lay out a
   driver's static ones. `DDT$L_FDT` is vaxpunk's: the driver's FDT
   routine, which checks the function itself, not VMS's table of masks.
2. **`IOC$GL_DEVLIST` starts with the disks**, `DKA`, `DKB` and `MDA`,
   whose DDBs and UCBs are in `qio.mar`, as VMS's boot device's are in
   its `DEVICEDAT`. Each other driver links its DDBs at boot
   (`IOC$LINK_DDB`): `TTY$INIT` the console's `OPA`, `FTA`, `TNA` and
   `VTA`, `MB$INIT` the mailboxes' `MBA`, `NET$INIT` the network's
   `BG`. `IOC$INIT` then calls each unit's `DDT$L_UNITINIT`: a PAL disk
   is online if `GETMAXBLOCK` gives it blocks, the ramdisk once
   `MD$CREATE` makes it.
3. **One search, VMS's.** `IOC$SEARCHDEV` translates a name as
   `$ASSIGN` always did, then parses it, letters then digits, and finds
   the DDB and the unit: `DK0`, `DKA`, `OP0`, `MBA02` work as on VMS; a
   name that isn't letters and digits is `SS$_IVDEVNAM`, one no device
   has `SS$_NOSUCHDEV`. `$ASSIGN`, `$GETDVI`, `$CREMBX`, `$BRKTHRU`,
   `IO$M_TT_CONNECT`, `$CREPRC`'s terminal owner and the file system's
   `FIL$SELECT` use it. `IOC$CVT_DEVNAM` makes a device's name,
   `_DDCU:`, from its DDB and unit.
4. **Templates and clones.** `IOC$CLONE_UCB` copies a template into
   pool, online, `UCB$V_DELETEUCB`, with no channels, numbered the first
   number past the template's `UCB$W_UNIT_SEED` that no unit of its DDB
   has, up to 9999 and round again; `IOC$LINK_UCB` links it in order and
   `IOC$DELETE_UCB` takes off and frees a marked unit no channel has.
   `$ASSIGN` to a template clones a unit for the channel, needing
   `NETMBX` for a network device's, and the driver's
   `DDT$L_CLONEDUCB` sets it up or refuses it: `BG0:` gives a network
   unit with no socket yet, which `IO$_SETMODE` makes; `TNA0:` and
   `VTA0:` refuse, since a TELNET terminal is made with its line by
   `IO$_TTY_PORT`, which clones it itself. The mailboxes' template isn't
   on the list, as VMS's; `$CREMBX` clones `MBAn` from it, `MBA2` the
   operator's at boot. `FTA0:` isn't a template, as on VMS: its
   `IO$_SETMODE` clones `FTAn` and moves the channel there.
5. **`IOC$CANCEL` goes by channel.** For each channel it ends the IRPs
   waiting on its UCB, then calls the driver's `DDT$L_CANCEL` with the
   channel, the IRP that was at the head, the channel's bit and the
   status; for a channel that goes, the UCB counts it no more first, and
   a unit marked for deletion whose last channel it was is deleted
   after. Every UCB counts its channels, `UCB$W_REFC`, the disks' too.
6. **`$GETDVI` answers from the UCB**, copied at `IPL$_SYNCH`, and from
   the VCB only for the volume's items; `MOUNT` and `DISMOUNT` keep
   `DEV$M_MNT`, `DEV$M_SWL` and `DEV$M_ALL` in the disk's UCB, as on
   VMS. A terminal's `DEV$M_DET` and `DEV$M_RED` are its
   `UCB$L_DEVCHAR2`'s, and a redirected one's `$ASSIGN` goes to its
   virtual terminal by that bit. `$DEVICE_SCAN` walks the database; its
   context is the DDB and the number past the last unit found, so a unit
   that goes between calls loses no other. `SHOW DEVICES` lists the
   disks, the terminals, then the rest but mailboxes and `DEV$M_MBX`
   devices unless named, under VMS's headings, and `/FULL` says a
   device's `DEV$` bits as VMS's does.

## Alternatives considered

| Option | Why not |
| --- | --- |
| One list of UCBs, without DDBs | A unit's name would still be kept in each UCB, and the search couldn't drop a controller's letter or a unit's number as VMS's does; every later VMS structure (SHOW DEVICES by controller, `SYSGEN CONNECT`, allocation classes) hangs off the DDB. |
| Link the DDBs statically across modules | One module's last DDB would name the next module's first, so the order of the database would be spread over four files; a driver linking its own at init is what loading a driver does. |
| Keep each driver's cancel routine walking a mask of channels | `IOC$CANCEL` had to name every driver; with a DDT per driver it calls the channel's, as VMS's `$CANCEL` does, and a new driver needs no change here. |
| Keep cloning `BGn` at `IO$_SETMODE` | TCP/IP Services clones at `$ASSIGN`, and a program that assigns `TCPIP$DEVICE:` and asks `$GETDVI` about the channel would see `BG0:` instead of its unit. |
| Make `FTA0:` a template too | VMS's isn't (`DVI$_STS` 0, offline); `PTD$CREATE` relies on its `IO$_SETMODE`. |
| Copy VMS's UCB offsets, fork block first | No code reads a UCB but the executive's, and the IRP queue at offset 0 keeps the drivers' queue walks as they are. The names and meanings are VMS's. |
| VMS's FDT tables of function masks | The drivers check their functions in one routine each; tables belong with loading drivers at run time ([t8y:/exec/qio#driver-structures](../map/exec/qio.md)). |

## Consequences

**What gets harder.**
- A clone is `UCB$W_SIZE` bytes of pool: a terminal's is 5.6 KB, as
  before.
- `SS$_IVDEVNAM` where a name with characters a device's can't have
  used to be `SS$_NOSUCHDEV`.
- A network unit needs `NETMBX`, as on VMS; a process created without
  it can't make a socket.
- The disks' `DVI$_REFCNT` counts channels now, so `SHOW DEVICES/FULL`
  of a disk shows a reference count, as VMS's does.
- `PCB$T_TERMINAL` holds 7 characters, VMS's 8 bytes counted: a
  terminal past `TNA999:` has its name cut there.

**What stays easy.**
- A new driver is a DDT, a DDB and its units, linked at its init.
- Programs see VMS's names: `_TNA12:`, `_MBA3:`, `_BG17:`, `_BG0:` in
  `TCPIP$DEVICE`, and `$DEVICE_SCAN` and `SHOW DEVICES MB` find every
  device.

**Follow-ups:** VMS's CRBs, IDBs and FDT tables, and drivers loaded at
run time (`t8y:/exec/qio#driver-structures`); `$ALLOC` and `$DALLOC`
with `UCB$L_PID` (`t8y:/exec/qio#allocate`); `UCB$L_MAXBLOCK` and
`DVI$_MAXBLOCK` (`t8y:/drivers/disk#ucb-items`); operation counts,
`UCB$L_OPCNT`; the null device, `NLA0:`; node names and allocation
classes with clusters.
