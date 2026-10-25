# What happens during boot

The boot sequence, from power on to the DCL prompt on the console, and
what the test programs you can run from there do, in plain words. It is
kept current: a PR that changes what boot does changes this file too (see
[AGENTS.md](../AGENTS.md)).

Booting goes through five programs, each starting the next one. Then the
executive starts the STARTUP process, which runs the site's startup
procedure, and then the console's, which asks for a username and a
password and runs DCL, which waits for commands.

## 1. Firmware, then Limine

QEMU's firmware finds the boot disk (`out/esp.img`) and starts Limine, a
bootloader. QEMU tells the firmware to boot from that disk alone, so the
firmware leaves the other devices, the other disks and the network, to the
OS. As [boot/limine.conf](../boot/limine.conf) says, Limine loads
three files into memory: the shim, the seL4 kernel and the root task.
Limine then jumps to the shim.

QEMU has a second disk, the system disk (`out/sysdisk.img`). It is a
Files-11 ODS-2 volume, the file system VMS uses, which
[vms/build.rs](../vms/build.rs) makes at build time: the VMS
executable files in `[SYSEXE]`, the shareable images they call,
`LIBRTL.EXE` and `LBRSHR.EXE`, the librarian's `LBR$` routines, in `[SYSLIB]`, and text files in `[SYSMGR]`, owned by
SYSTEM's UIC, `[1,4]`, which every user may read and run. The firmware
and Limine leave it alone; the root task reads it later.

## 2. The shim

The shim ([pal/shim/src/main.c](../pal/shim/src/main.c), `shim_main`) is a small C
program. Its job is to put the machine in the state seL4 expects at startup
([pal/shim/README.md](../pal/shim/README.md) has the details).

- It looks up the serial port's address in the hardware description (the
  DTB) so it can print. That's the `vaxpunk shim:` line.
- It copies the kernel, then the DTB, then the root task into physical
  memory, one after another.
- It sets up the MMU and jumps into seL4.

## 3. seL4

seL4, the microkernel, takes over the CPU and memory. It creates one
program, the root task, and gives it every permission ("capability") in the
system.

## 4. The root task, also called the PAL

The root task ([pal/src/main.c](../pal/src/main.c), `main`) plays
the role PALcode played on an Alpha: it's the "hardware" layer underneath VMS
([ADR-0002](adr/0002-root-task-is-the-pal.md)).

- It divides the RAM seL4 gave it (`memory_init`), as
  [ADR-0037](adr/0037-all-ram-and-address-spaces-filled-on-faults.md)
  decides: a share, a sixteenth of it, for its own objects, and the rest
  for VMS as page frames, numbered from 0, PFNs. To hold a capability for each of them, it moves to a CSpace
  of two levels, with the CNode seL4 made it at the top's slot 0.
- It maps the serial port and the real-time clock, and reads the clock once
  to get the boot time. Then it prints `hello from the root task`.
- It reads the startup parameters (`fwcfg_init`): the SYSGEN parameters
  `STARTUP_P1` to `STARTUP_P8`, 4 characters each. `run-qemu.sh` makes
  them from its `--p1` to `--p8`, as in `cargo run -p boot -- --p1=MIN`,
  and hands them to QEMU's firmware configuration device as the file
  `opt/vaxpunk/startup`. The root task finds that file in the device's
  directory and prints `STARTUP_P1-8: "MIN                             "`.
  Parameters not given are blank.
- It prints a list of the free memory blocks seL4 gave it, and
  `RAM: 8186 MB, 1964667 PFNs for the executive, 511 MB for the PAL`.
- It starts a clock thread that wakes up every 10 ms. Each wakeup becomes
  VMS's timer interrupt ([ADR-0004](adr/0004-interval-timer-is-a-pal-thread.md)).
  On each wakeup the PAL also checks the serial port for typed
  characters, and once the executive asks for it, raises the console
  receive interrupt when there are some.
- `disk_init` finds the disks, virtio block devices: unit 0, the system
  disk, and unit 1, the data disk, `out/datadisk.img`, and prints
  `disk 0: virtio-blk, 16384 blocks` and `disk 1: ...` for them. From then
  on the PAL can read the disks' blocks, by number, for itself and for the
  executive, and write the data disk's for the executive. It also
  notes the network device, virtio-net, if QEMU has one, and sets up
  the entropy device, virtio-rng, printing `entropy: virtio-rng`. The
  PAL reads random bytes from it for `$GET_ENTROPY`
  ([ADR-0031](adr/0031-entropy-from-virtio-rng.md)).
- `start_exec` reads `EXEC.EXE` from the system disk, as VMS's first
  bootstrap did: the home block, the index file, the top directory,
  `[SYSEXE]`, then the file (`f11_boot_file`). It loads it, and creates
  the restart parameter block (RPB), a page describing memory (how many
  PFNs there are, the first one free, and the first S0 page past
  `EXEC.EXE`), the boot time and the startup parameters. With a network device it also makes the port's 17 pages
  for the executive, at `0x4FF00000`, and puts their address in the RPB.
  It then starts EXEC in kernel mode at IPL 31, with R11 pointing at the
  RPB.
- `start_tcpip` starts the TCP/IP component, if there is a network
  device ([DESIGN-0003](design/0003-tcpip-port.md)): `tcpip.elf`, lwIP
  and a virtio-net driver, which the root task carries inside itself. It
  copies it into a 2 MB page, gives it an address space with the port's
  pages, the device's registers and the console, and a few capabilities:
  its own notification, the PAL's, and the device's interrupt. It runs
  above the executive's threads and below the PAL. It sets up the device
  and prints `tcpip: lwIP 2.2.1 on virtio-net, MAC ...`, marks the port
  ready, and waits: for the executive's doorbell, the device's interrupt
  or the clock's tick, which the PAL's clock thread now gives it too.
- After that it loops in `serve`: it handles the executive's PAL calls
  (MTPR, SWPCTX, REI, CHMx…), page faults and clock ticks
  ([DESIGN-0001](design/0001-pal-interface.md)). It only stops on a halt or
  a fatal error.

## 5. The executive, EXEC.EXE

The executive is the VMS kernel, written in MACRO-32, with one module in
BLISS-64: `$NUMTIM`
([vms/exec/numtim.b64](../vms/exec/numtim.b64)), which `vbliss`
compiles.
`EXEC$START` ([vms/exec/exec.mar](../vms/exec/exec.mar)):

- fills in the system control block (handlers for traps, mode changes and
  software interrupts, plus the timer and the console receive interrupt)
- starts memory management (`MMG$INIT`): the PFN database, in S0 past
  `EXEC.EXE`, 8 bytes for each PFN, and the pool; then lets user-mode code
  use the system service vector
- starts the scheduler (`SCH$INIT`)
- makes the system's logical name directory, `LNM$SYSTEM_DIRECTORY`, a
  table of tables, with the system's table, `LNM$SYSTEM_TABLE`, in it, and
  the names that stand for tables: `LNM$SYSTEM`, and `LNM$FILE_DEV`, where
  file and device names are looked up, the process's table and then the
  system's (`LNM$INIT`)
- starts the system time (`EXE$INITTIM`)
- makes the console's unit control block, `OPA0:`, the first terminal,
  with its characteristics, a VT100's 80 by 24, and an empty type-ahead
  buffer, where typed characters wait until a program reads them, and
  turns on the console receive interrupt (`TTY$INIT`)
- makes the operator's mailbox, `MBA2:`, a permanent one, which OPCOM
  will read and where `$SNDOPR` writes the messages for the operators
  (`MB$INIT`,
  [ADR-0042](adr/0042-opcom-reads-the-operators-mailbox.md))
- finds the port in the RPB (`NET$INIT`); its interrupt, at IPL 21, and
  the software interrupt it requests, at IPL 6, are in the SCB too
- prints `%EXEC-I-START … free pages`
- mounts the system disk, `DKA0:`: it reads the volume's home block and
  the index file's header, which says where every other file's header is
  (`FIL$MOUNT`), handing each read to the disk's driver (`DK$STARTIO`) in
  an I/O request packet, as the file system always does, defines
  `DISK$VAXPUNK`, the volume's label after `DISK$`, as `_DKA0:` in
  `LNM$SYSTEM_TABLE`, and prints `%MOUNT-I-MOUNTED, VAXPUNK mounted on _DKA0:`
- defines the system's logical names for it, in `LNM$SYSTEM_TABLE`:
  `SYS$SYSDEVICE` is `DKA0:`, `SYS$DISK`, the default device, is
  `SYS$SYSDEVICE:`, `SYS$SYSTEM`, where the images are, is
  `SYS$SYSDEVICE:[SYSEXE]`, `SYS$LIBRARY` and `SYS$SHARE`, where the
  shareable images are, are `SYS$SYSDEVICE:[SYSLIB]`, `SYS$MANAGER`, where the system manager's
  files are, is `SYS$SYSDEVICE:[SYSMGR]`, and `SYS$SCRATCH`, where
  `CONVERT` puts its work file, is `SYS$DISK:[]`, the default directory
- lowers IPL to 0 and creates the process `STARTUP`, which runs
  `DCL.EXE`, with the logical name `SYS$INPUT` standing for the procedure
  `SYS$MANAGER:SYSTARTUP_VMS.COM`, and `SYS$OUTPUT` and `SYS$ERROR` for
  the console, `_OPA0:`, in its process table. STARTUP inherits the
  swapper's username, `SYSTEM`, its UIC, `[1,4]`, and its privileges,
  every one, which `SCH$INIT` gave it, and passes them on to the
  processes it creates
- then becomes the swapper (process 1). The swapper cleans up deleted
  processes and sleeps the rest of the time. Once STARTUP is gone, it
  creates the console's process, which logs you in (section 6), and does
  again each time that process is deleted.

## 6. STARTUP, logging in, and DCL

DCL ([vms/sysexe/dcl.mar](../vms/sysexe/dcl.mar)) is the command
interpreter. It is linked high in P1, which tells the executive it is one
([ADR-0006](adr/0006-cli-in-p1-runs-images-in-its-process.md)):

- When STARTUP starts (`EXE$PROCSTRT`), the executive makes its stacks,
  reads `SYS$SYSTEM:DCL.EXE`, which is `DKA0:[SYSEXE]DCL.EXE`, from the
  disk, its header (`FIL$OPENFILE`) and then each section into its pages
  (`FIL$READIMG`), which loads it into P1, and calls it in supervisor
  mode.
- DCL opens a channel to `SYS$INPUT`, and `$GETDVI` says it is no
  terminal, so DCL takes its commands from it as a command procedure,
  `@SYS$INPUT`: `DKA0:[SYSMGR]SYSTARTUP_VMS.COM`, the site's own
  startup, as VMS's STARTUP process runs it once at boot. First, as VMS's
  `STARTUP.COM` does, it sets `P1` to `P8` from `F$GETSYI("STARTUP_P1")`
  to `F$GETSYI("STARTUP_P8")`, blanks trimmed and in upper case, so the
  rest of it can test them. `EXEC$START` copied them from the RPB to
  `SGN$GB_STARTUP_P1`, where `$GETSYI` finds them. Next it defines the system
  logical name `TCPIP$DEVICE` as `_BGA0:`, the network's template device,
  which a program assigns a channel to for a socket, as TCP/IP Services'
  `TCPIP$STARTUP.COM` defines it. Then `INITIALIZE/SIZE=2048 MDA0: RAM` makes the
  ramdisk, 2,048 blocks, empty, and `MOUNT/SYSTEM MDA0: RAM` mounts it for every
  process and prints `%MOUNT-I-MOUNTED, RAM mounted on _MDA0:`. Next it makes
  `SYS$SYSROOT` a search list, as a VMScluster's common system disk has
  it: `SYS$SPECIFIC`, this system's own root, is the ramdisk, `MDA0:`,
  and `SYS$COMMON`, the root every system shares, is the system disk.
  `CREATE/DIRECTORY` makes `MDA0:[SYSEXE]`, `MDA0:[SYSLIB]` and
  `MDA0:[SYSMGR]`, and `SYS$SYSTEM` becomes `SYS$SYSROOT:[SYSEXE]`,
  `SYS$LIBRARY` and `SYS$SHARE` `SYS$SYSROOT:[SYSLIB]`, `SYS$MANAGER`
  `SYS$SYSROOT:[SYSMGR]`, `SYS$HELP`, where `LBR$OUTPUT_HELP` looks for
  help libraries, `SYS$SYSROOT:[SYSHLP]`, `SSL3$CERTS`, where SSL3 finds `CERT.PEM`,
  the CAs it trusts, `SYS$COMMON:[SSL3.CERTS]`, and `SYS$DISK`, the
  default device of a process that hasn't one of its own, `SYS$SYSROOT:`. RMS looks for a file in the ramdisk's
  directory first and then in the system disk's, and makes a new one in
  the ramdisk's, so `COPY` to `SYS$MANAGER:` works though `DKA0:` is
  write locked, and a file there hides the system disk's of the same
  name until the system stops. So `COPY` puts a copy of `SYSUAF.DAT`,
  the users, which the build wrote in `DKA0:[SYSEXE]`, in `MDA0:[SYSEXE]`,
  where `AUTHORIZE` and `SET PASSWORD` can change it, and `SET
  PROTECTION` lets only SYSTEM read it; and a copy of `RIGHTSLIST.DAT`,
  the rights database: the identifiers, a name each, and who holds them,
  which `AUTHORIZE` can change there and anyone may read
  ([ADR-0040](adr/0040-acls-in-file-headers-and-rights-identifiers.md)).
  Before those copies, `RUN/DETACHED/PRIORITY=6/PROCESS_NAME=OPCOM
  SYS$SYSTEM:OPCOM` creates the process `OPCOM`, which runs `OPCOM.EXE`
  at priority 6, above STARTUP's 4: DCL runs `RUNDET.EXE` for `RUN`'s
  qualifiers for a new process, as VMS's does, which creates it with
  `$CREPRC` and prints `%RUN-S-PROC_ID,
  identification of created process is` and its PID, as VMS's
  `STARTUP.COM` starts it. OPCOM assigns a channel
  to `MBA2:` and writes on the console, with `$BRKTHRU`, that it is an
  operator terminal, `Operator _OPA0: has been enabled, username SYSTEM`,
  and for which classes of messages, each under OPCOM's banner,
  `%%%%%%%%%%%  OPCOM  21-OCT-2026 10:00:00.00  %%%%%%%%%%%`. Then it
  waits for a message in its mailbox. The ramdisk also holds TCP/IP's two
  files: the hosts database, `TCPIP$HOST.DAT`, which the first `TCPIP
  SET HOST` or `SHOW HOST` makes, and the saved network configuration,
  `TCPIP$CONFIG.DAT`. So the only hosts a boot knows, and its fixed
  address if it has one, are what `TCPIP SET HOST`, `SET CONFIGURATION
  INTERFACE` and `SET ROUTE /PERMANENT` lines in this procedure save
  ([ADR-0025](adr/0025-hosts-database.md)). Then `TCPIP SET
  NAME_SERVICE /SERVER=8.8.8.8 /SYSTEM` defines the system logical name
  `TCPIP$BIND_SERVER000` as `8.8.8.8`, Google's public DNS server, which
  every program asks for a host the hosts database hasn't, and `nslookup`
  asks for any ([ADR-0026](adr/0026-name-service-and-nslookup.md)). `TCPIP START COMMUNICATION`
  opens a UDP socket on `TCPIP$DEVICE:` and, with ioctls on
  it, sets the interface's address, mask and gateway as `TCPIP
  SET CONFIGURATION INTERFACE` and `SET ROUTE /PERMANENT` saved them
  on the ramdisk, or asks a DHCP server for them if the saved settings
  say `DHCP` or nothing is saved, waiting up to 10 seconds for its answer.
  If the DHCP server offered a DNS server, as QEMU's offers `10.0.2.3`, it
  defines `TCPIP$BIND_SERVER000` as that one instead of `8.8.8.8`. It prints
  `%TCPIP-I-SET, WE0: ...`, and creates the process `TCPIP$TELNET`, which runs `TELNETD.EXE` and waits
  on TCP port 23 for `SET HOST` from another vaxpunk. Then it mounts the
  data disk, whatever its label, if an `INITIALIZE DKB0:` wrote a volume
  there, at this boot or an earlier one: `MOUNT/OVERRIDE=IDENTIFICATION
  DKB0:` mounts it for STARTUP alone, `F$GETDVI("DKB0:", "VOLNAM")` reads
  its label, `DISMOUNT DKB0:` dismounts it, and `MOUNT/SYSTEM DKB0:
  label`, which must be given the label, mounts it for every process.
  Each `MOUNT` prints `%MOUNT-I-MOUNTED, label mounted on _DKB0:`. On a
  blank disk the first prints `%SYSTEM-W-NOHOMEBLK` instead, and the
  disk stays unmounted. Its last
  command is `LOGOUT`, which, with no terminal, prints
  `  SYSTEM       job terminated at  9-OCT-2026 10:00:00.00`, as VMS's STARTUP
  does, and deletes the process.
- The swapper then creates the console's process, named after the
  console, `_OPA0:`, which runs `LOGINOUT.EXE`
  ([vms/sysexe/loginout.mar](../vms/sysexe/loginout.mar)) in user mode,
  with every privilege ([ADR-0029](adr/0029-loginout-and-sysuaf.md)).
  First it looks for its terminal in the Automatic Login Facility's file,
  `SYS$SYSTEM:SYSALF.DAT`, whose lines are a terminal and a username,
  `OPA0: SYSTEM`. `just boot` runs `cargo run -p boot -- --autologin`,
  which writes that line into `[SYSEXE]` on its copy of the system disk;
  then LOGINOUT logs the console in as `SYSTEM` without asking, and
  goes on from the welcome below. Without the file, or a line for the
  terminal, LOGINOUT prints the system logical name `SYS$ANNOUNCE`,
  which `SYSTARTUP_VMS.COM` defined as `@SYS$MANAGER:ANNOUNCE.TXT`, so
  the lines of that file, then `Username:` and reads a name, then `Password:` and
  reads the password without echoing it. It finds the user's record in
  `SYS$SYSTEM:SYSUAF.DAT`, an indexed file keyed by username, with RMS,
  and has `$HASH_PASSWORD` hash the password with the record's salt
  and the username, SHA-256 4096 times over, to compare it with the hash
  in the record. The build wrote one user who may log in, `SYSTEM`, whose
  password is `MANAGER`. A wrong username or password prints `User
  authorization failure`, and a wrong password is counted in the record;
  after three failures LOGINOUT exits and the swapper starts another.
  With both right, LOGINOUT names the process after the user, makes the
  record's default directory the process's, `[SYSMGR]` for SYSTEM, and
  defines `SYS$DISK`, `SYS$LOGIN` and `SYS$SCRATCH` from it, so
  SYSTEM's `SHOW DEFAULT` says `SYS$SYSROOT:[SYSMGR]`, `=   MDA0:[SYSMGR]`
  and `=   DKA0:[SYSMGR]`, as on VMS. It makes the process's rights list:
  `INTERACTIVE`, `LOCAL`, or `REMOTE` on a TELNET terminal, and each
  identifier `RIGHTSLIST.DAT` says the user's UIC holds, with
  `$FIND_HELD`. Then, with `$CMKRNL`, it calls
  `EXE$LOGIN` in the executive, which loads `DCL.EXE` into P1 and gives
  the process the record's UIC, privileges and priority, and the rights
  list. LOGINOUT prints
  `SYS$WELCOME`, or without one `Welcome to vaxpunk arm64 Operating
  System, Version V0.1`, the version from `$GETSYI`'s `SYI$_VERSION`, and how many failures there were since the last
  login, and exits. The executive throws its pages away, as for any
  image that exits, and calls DCL.
- DCL opens a channel to `SYS$INPUT`, the console, `OPA0:`. Its first
  command is `@SYS$MANAGER:SYLOGIN`, which runs
  `DKA0:[SYSMGR]SYLOGIN.COM`, as VMS runs it at each login: it defines
  the global symbol `HOME`, a command that goes back to
  `SYS$SYSROOT:[SYSMGR]`, and `NSLOOKUP`, a foreign command that runs
  `SYS$SYSTEM:TCPIP$NSLOOKUP.EXE`. On `OPA0:` it then runs `TTSIZE`,
  which finds the console window's size: a serial line can't send it, as
  a TELNET client sends NAWS, so `TTSIZE` moves the cursor to row 999,
  column 999, asks the terminal where it is with `ESC [6n`, and makes the
  row and column it answers the terminal's page and width. A terminal
  that doesn't answer in 3 seconds keeps 80 by 24. The second is `@SYS$LOGIN:LOGIN`, the
  user's own `LOGIN.COM`, if there is one. SYSTEM's,
  `DKA0:[SYSMGR]LOGIN.COM`, goes on only if `F$MODE()` says
  `INTERACTIVE`, which it does since DCL's `SYS$INPUT` is a terminal, and
  prints `SYSTEM on _OPA0:,` and the time, with `F$FAO`, `F$GETJPI`,
  `F$GETDVI` and `F$TIME`. Then DCL reads a line with the `$` prompt: `$QIOW`
  hands the read to the console's driver, which writes the prompt and
  holds the request until a line is typed, and DCL waits for its event
  flag ([ADR-0013](adr/0013-qio-irps-and-drivers.md)). That's where the
  boot ends: the CPU idles, taking clock ticks, until you type something.
  Each key you type comes in through the console receive interrupt, and
  the driver echoes it and edits the line; a carriage return completes
  the read, and DCL gets the line.
- `@file` runs a command procedure the same way: DCL reads the whole file,
  `file.COM` if it has no type, with RMS's `$OPEN` and `$GET`, into a
  buffer in P1, and closes it, so nothing stays open while the images it
  runs come and go. Then it takes its commands from there, the lines that
  start with `$`, until the end or an `EXIT`, before it prompts again. The
  words after the file name are the local symbols `P1` to `P8`.
  `name = expression` makes a local symbol, which the procedure that made
  it and those it calls see, and `name == expression` a global one;
  `'name'` in a command stands for its value. `IF`, block `IF`s, `GOTO`,
  `ON`, the lexical functions and `OPEN`, `READ`, `WRITE` and `CLOSE`
  work in procedures as in VMS's, and `$STATUS` holds the last command's
  status: one that is an error ends the procedure, as VMS's default `ON
  ERROR THEN EXIT` does. The lines after a command that runs an image, up
  to the next that starts with `$`, are the image's input: DCL writes them
  to a mailbox, with an end of file after them, and makes `SYS$INPUT` that
  mailbox until the image exits, so `CREATE` and `LIB$GET_INPUT` read them
  as they would lines typed at the terminal; `SYS$COMMAND`, which
  `$CREPRC` defines with `SYS$INPUT`, stays the terminal.
- DCL parses each command with its command tables, `DCL$TABLES`, which
  `vms/build.rs` compiled from the verbs' definitions in
  `vms/cld/*.cld` with vcdu, and linked into `DCL.EXE`. The parser,
  `CLI$$DCL_PARSE` in `vms/sysexe/lib/cli.mar`, matches the verb on
  its first 4 characters, then its parameters and its qualifiers, such as
  `/LOG`. It prompts for a parameter the command needs but lacks, `_From: `,
  and reports a mistake as VMS does: `%DCL-W-IVQUAL, unrecognized
  qualifier - check validity, spelling, and placement`, then the word in
  backslashes. A verb that runs an image passes it the parse, which the
  image reads with `CLI$GET_VALUE` and `CLI$PRESENT`; DCL does the others
  itself, and reads their values and qualifiers the same way.
- `RUN image` reads the image from `SYS$SYSTEM:` and loads it into the same
  process's P0 (`$IMGACT`) and runs it in user mode. Every image but DCL
  calls `LIB$PUT_OUTPUT` and the other run-time routines in
  `LIBRTL.EXE`, a shareable image: the image activator reads
  `SYS$SHARE:LIBRTL.EXE` too, loads it into P0 after the image, at the
  next 64 KB, moves the addresses it holds there (`SHRACT`), and puts its
  routines' addresses where the image looks for them (`SHRLIST`,
  [ADR-0028](adr/0028-shareable-images.md)). An image that calls the
  `LBR$` routines gets `SYS$SHARE:LBRSHR.EXE` the same way. When the image
  exits, the executive throws away its pages, and `LIBRTL`'s, and the channels and files
  it opened, and calls DCL again with the exit status. DCL prints a
  message if the status is an error, then the prompt again.
- `DIRECTORY` (`DIR`) runs `DIRECTORY.EXE`, which lists files with RMS's
  `$PARSE` and `$SEARCH`: those in the default directory, `[SYSMGR]` until
  `SET DEFAULT` changes it (`$SETDDIR`, and `SYS$DISK` for the device;
  `SHOW DEFAULT` shows it), or the ones it is given, `DIR [SYSEXE]`,
  `DIR [000000]`, `DIR SYS$SYSTEM:`: RMS translates a device that is a
  logical name. `TYPE file` runs
  `TYPE.EXE`, which reads the file with `$OPEN` and `$GET` and writes it
  on the console. `EDIT file` runs `EDIT.EXE`, EDT's line mode: it reads
  the file the same way and, at its `*` prompt, types the lines you ask
  for, and `INSERT`s, `DELETE`s and `REPLACE`s them; lines to insert end
  with CTRL/Z. `CHANGE` goes to keypad mode, which shows the file on the
  screen, reads a key at a time (`$QIO` with `IO$M_NOFILTR`) and changes
  the text, until CTRL/Z. `EXIT` writes the buffer to the file's next
  version with `$CREATE` and `$PUT`, which works on the ramdisk only, and
  `QUIT` leaves without writing. `DEFINE`,
  `DEASSIGN` and `SHOW LOGICAL` make, delete
  and translate logical names (`$CRELNM`, `$DELLNM`, `$TRNLNM`), and
  `SHOW LOGICAL` alone lists them. `SHOW PROCESS` and `SHOW SYSTEM` run
  `SHOW.EXE`, which prints the date with `$ASCTIM`, asks `$GETJPI` about
  this process, and `$GETSYI` how long the system has been up, then `$GETJPI` with a wildcard for a line
  per process: its PID, name, state, priority and image. `SET
  PROCESS/PRIVILEGES` runs `SET.EXE`, which enables or disables them with
  `$SETPRV`, and `SHOW PROCESS/PRIVILEGES` lists them. `SET
  PROTECTION=(W:R) file` and `SET FILE/OWNER_UIC=[200,1] file` run
  `SET.EXE` too, which changes the protection and the owner in the
  file's header, and `DIRECTORY/OWNER/PROTECTION` shows them. `HELP` runs
  `HELP.EXE`, which writes the topic asked for from
  `SYS$HELP:HELPLIB.HLB` and prompts for more, `Topic?`.
  `MCR AUTHORIZE` runs `AUTHORIZE.EXE`, which reads `SYSUAF.DAT` and
  does the command after it, or each one after its `UAF>` prompt:
  `ADD JOE /PASSWORD=secret /UIC=[200,1]` makes a user from the `DEFAULT`
  record, with the qualifiers' values, `MODIFY`, `REMOVE`, `SHOW` and
  `LIST` change, delete, show and list them. Only SYSTEM may: the file is
  SYSTEM's alone. `SET PASSWORD` runs `SETPWD.EXE`, which asks for the
  old password and the new one twice and writes its hash in the user's
  record; the executive gives that image the `SYSPRV` privilege while it
  runs, so that any user may change their own. `LOGOUT` prints
  `  SYSTEM       logged out at`, the time, and deletes the process; the
  swapper then starts LOGINOUT on the console again, which asks for the
  next username.
- `INITIALIZE/SIZE=n MDA0: label`, which `SYSTARTUP_VMS.COM` runs at boot,
  runs `INIT.EXE`, whose `$INIT_VOL` makes the ramdisk, `MDA0:`, n blocks of memory
  (as many as the last time without `/SIZE`, 1,024 at first), and writes an empty volume on it,
  SYSTEM's, `[1,4]`, unless `/OWNER_UIC` says otherwise. Each file made
  there is its maker's, which may do anything with it; its group may read
  and run it, the world nothing, unless `SET PROTECTION` says otherwise.
  `MOUNT/SYSTEM MDA0: label` runs `MOUNT.EXE`, whose `$MOUNT` mounts it,
  defines `DISK$label`, and the logical name given after the label if
  there is one, as `_MDA0:` in the system table, and prints
  `%MOUNT-I-MOUNTED, label mounted on _MDA0:`. Without `/SYSTEM` the
  volume is the job's, as on VMS: the disk is allocated to the job's
  master process, its names go in that process's table, another job's
  `$ASSIGN` gets `%SYSTEM-W-DEVALLOC` and RMS `%RMS-E-DNR`, and
  `FIL$RUNDOWN` dismounts it when the master process goes. Then `COPY` makes files
  there (`COPY WELCOME.TXT MDA0:[000000]`, from `[SYSMGR]`, the default), `DELETE` deletes them
  (`DELETE MDA0:[000000]WELCOME.TXT;1`), and `DIRECTORY`, `TYPE` and `RUN`
  read them as they do the system disk's.
- `INITIALIZE DKB0: label` and `MOUNT DKB0: label` do the same on the
  data disk, `DKB0:`, a disk image on the host which the PAL writes with
  `WRITELBLK` ([ADR-0012](adr/0012-data-disk-writable-files-11.md)), as
  big as the disk: `run-qemu.sh` makes it 64 MB, and `$INIT_VOL` asks the
  PAL its size with `GETMAXBLOCK`
  ([ADR-0043](adr/0043-volumes-the-size-of-their-disk.md)). What
  `COPY` and `DELETE` do there is still there at the next boot, whose
  `SYSTARTUP_VMS.COM` mounts it, and `ods dir out/datadisk.img '[000000]'`
  lists it on the host.
- `CREATE/DIRECTORY DKB0:[SUB.DEEP]` runs `CREATE.EXE`, whose
  `$CREATE_DIR` walks the directories from the MFD, as RMS does to find
  a file, and makes each one that isn't there, `SUB.DIR;1` in
  `[000000]`, then `DEEP.DIR;1` in `[SUB]`: an empty directory is a
  header marked a directory and one block (`FIL$MKDIR`). `COPY`,
  `DIRECTORY` and `SET DEFAULT` then take `[SUB.DEEP]` as any other.
  `DELETE` refuses a directory with files in it (`%RMS-E-MKD`).
- `CREATE/LOG DKB0:[SUB.DEEP]NOTE.TXT`, without `/DIRECTORY`, runs the
  same `CREATE.EXE`, which `$CREATE`s a text file there, variable length
  records with carriage return carriage control, then reads lines from
  `SYS$INPUT` and `$PUT`s each, until CTRL/Z, echoed as `*EXIT*`.
- `DISMOUNT DKB0:` runs `DISMOUNT.EXE`, whose `$DISMOU` makes the file
  system forget the volume, and deletes the logical names `MOUNT`
  defined, unless a process has a file on it open, so it can be mounted
  again, or initialized afresh. The system disk can't
  be dismounted (`%SYSTEM-F-DEVACTIVE`).
- `SHOW DEVICES` (`SHO DEV`) lists the disks, mounted or not, with the
  errors the disk's driver counted in its UCB, and their volumes' labels
  and free blocks, then the console, `OPA0:`, as VMS
  does: it finds them with the `$DEVICE_SCAN` system service and asks
  `$GETDVIW` about each. The free blocks are the count in the volume's
  VCB, which the first `SHOW DEVICES` after `MOUNT` takes from the
  storage bitmap (`FIL$FREEBLOCKS`). `SHOW DEVICES DK` lists only the
  devices whose names start with `DK`.
- `REQUEST "text"` runs `REQUEST.EXE`, which sends the text, after the
  terminal's name, `_OPA0:, text`, to OPCOM with `$SNDOPR`. The
  executive writes it to `MBA2:` after a header that says who sent it,
  and OPCOM writes it on the console under its banner, after `Message
  from user SYSTEM`. `/TO=(DISKS,OPER12)` sends it to those classes of
  operators only; the console is one of every class but `REPLY` and
  `SOFTWARE`, as on VMS. `RUN/DETACHED` runs any
  image in a detached process, as `SYSTARTUP_VMS.COM` runs OPCOM; `RUN`
  with any other qualifier for a new process, `/PROCESS_NAME` and the
  rest, makes a subprocess instead, unless `/UIC` makes it detached. A
  subprocess goes when the process that made it does: that process's
  rundown, `EXE$RUNDOWN`, deletes it first. Either kind takes the name,
  base priority, `SYS$INPUT`, `SYS$OUTPUT`, privileges, UIC and I/O
  quotas the other qualifiers give.
- A command that fails prints VMS's message for the status, when DCL
  knows it: `DIR DKB0:` looks in `DKB0:[SYSMGR]`, the default directory
  on that disk, and prints `%RMS-E-DNF, directory not found`.
- `SHUTDOWN`, a symbol SYLOGIN.COM defines for `@SYS$SYSTEM:SHUTDOWN`,
  runs `SHUTDOWN.COM`, which ends with `MCR OPCCRASH`. `OPCCRASH`, with
  the `CMKRNL` privilege, halts the processor in kernel mode. The root task prints `%PAL-I-POWEROFF`, and `run-qemu.sh`'s
  console filter, `scripts/serial-filter.py`, sees it and stops QEMU.
- CTRL/Y is an AST of DCL's: when DCL starts, and before each image it
  runs, it asks the console's driver for one with `IO$_SETMODE`
  ([ADR-0014](adr/0014-ctrlc-ctrly-asts.md)). CTRL/Y while an image runs
  makes the console print `*INTERRUPT*` and end the image's reads with
  `SS$_CONTROLY`, and queues the AST. The executive delivers it on top of
  the image, in supervisor mode, leaving the image as it was on the kernel
  stack, and DCL prompts inside it. `CONTINUE` returns from the AST, and
  the image goes on; a command that runs another image throws the stopped
  one away first. CTRL/Y at the prompt ends the line being typed, and any
  command procedure. CTRL/C is the same, unless a program asked for a
  CTRL/C AST of its own: then the console prints `*CANCEL*` and that AST
  comes instead.

- `TCPIP` runs `TCPIP.EXE`, which parses TCP/IP Services' commands with
  its own tables, or prompts `TCPIP>` for them. `SET INTERFACE WE0
  /HOST=address /NETWORK_MASK=mask`, or `/DHCP`, `SET ROUTE /DEFAULT
  /GATEWAY=address` and `SHOW INTERFACE` sense and set the interface's
  address, mask and gateway with TCP/IP Services' ioctls (`SIOCSIFADDR`,
  `SIOCGIFADDR`, `SIOCADDRT` and the rest), `$QIOW` with `IO$_SETMODE`
  and `IO$_SENSEMODE` on a UDP socket on `TCPIP$DEVICE:`, the network's
  port driver. `SET CONFIGURATION INTERFACE WE0` and `SET ROUTE /DEFAULT
  /GATEWAY=address /PERMANENT` save them on the ramdisk for `START
  COMMUNICATION` instead. `SET HOST name /ADDRESS=address`, `SET NOHOST`
  and `SHOW HOST` keep the hosts database there, and `PING`, `TELNET`,
  `SET HOST` and `COPY/HTTP` find a name in it with `HOST_ADDR`. `PING host` sends an ICMP echo request each second
  on a raw ICMP socket and prints each reply.
  `SET HOST address` runs `RTPAD.EXE`, and so does `TELNET address`,
  which connects to port 23 there:
  the other side's `TELNETD` makes the connection a terminal, `_TNA1:`
  say, which the terminal driver there speaks Telnet on, and creates a
  process named after it, running LOGINOUT with it as its input and
  output, which asks for a username and a password there, as on the
  console, and gives the process DCL.
  That terminal offers to echo, so RTPAD goes a character at a time:
  every key goes there, CTRL/Y too, and comes back echoed, so EDIT's
  keypad mode works there as on the console, on a screen of this
  terminal's width and page, which RTPAD sends. `LOGOUT` there prints
  `%REM-S-END` here. CTRL/] gives RTPAD's `TELNET>` prompt, where
  `CLOSE` ends the session too and a blank line goes back to it.
  `TELNET address port`, or `/PORT=port`, connects to that port instead,
  a line at a time unless what listens there offers to echo, until CTRL/Z. `RUN TCPTEST` connects
  to a server on the host and accepts a connection from it.

The system disk is read only, and the ramdisk is gone when the system
stops, with the users `AUTHORIZE` added and the passwords `SET PASSWORD`
changed.

## 7. STARTUP and the test processes

`RUN STARTUP` at the prompt starts the tests
([vms/sysexe/startup.mar](../vms/sysexe/startup.mar)). STARTUP
runs inside your process; the programs it starts are processes of their own,
and each one in [vms/sysexe/](../vms/sysexe/) tests one
executive feature:

- STARTUP allocates 4 pages, writes them, checks what it reads back and
  frees them.
- **SLEEPER** has a higher priority than STARTUP, so it runs before STARTUP
  continues. STARTUP deletes it later.
- **PING/PONG** take turns sending signals to each other through shared
  event flags. PONG then sets a third flag of theirs, which STARTUP waits
  for.
- **SVCTEST** checks the status that each system service returns, and
  reads the system disk's home block by its LBN with `$QIOW`. A `$QIOW`
  from ARM64 with a buffer above 4 GB gets `SS$_ARG_GTR_32_BITS` from the
  service dispatcher. It turns fixed times into text with `$ASCTIM` and
  into numbers with `$NUMTIM`, and checks the result. It fills two
  buffers with `$GET_ENTROPY` and checks that they differ. It takes
  privileges away and back with `$SETPRV`, and makes OUTSIDER, a process
  in another UIC group it may touch only with `WORLD`. It starts a
  second SLEEPER, NAPPER, and makes it exit with `$FORCEX` before its
  image runs (status `217C`). It ends by `$FORCEX`ing itself, so that
  its exit handler, declared with `$DCLEXH`, is what prints `SVCTEST: ok`.
  It also starts:
  - **SNOOP**, which reads kernel memory and should die with an access
    violation: no condition handler takes it, so the executive prints
    `%SYSTEM-F-ACCVIO` and the image exits with status `1000000C`, the
    condition with the bit that says its message was printed
  - **USURP**, which runs a privileged instruction in user mode and should
    die the same way with a reserved-instruction error (status `1000043C`)
- **HOG** spins without ever waiting. **NUDGE** can only run if the timer
  interrupt takes the CPU away from HOG, so it tests preemption.
- **TIMETEST** checks reading the time, timers and scheduled wakeups.
- **ASTTEST** checks ASTs: one `$DCLAST` declares, which runs as the
  service returns; one held back by `$SETAST` until ASTs are enabled
  again; a timer's, whose routine wakes it from `$HIBER`; a `$QIO`'s,
  once the I/O is done; and one that comes while it computes, after
  which its registers are as they were.
- **MBXTEST** checks mailboxes: it makes one with a logical name, writes
  to it and reads back, fills it, reads it on channels from a second
  `$CREMBX` and from `$ASSIGN` by that name, and starts **MBXCHILD**, a
  subprocess whose image isn't there, with the mailbox's unit for its
  termination mailbox: it reads the message MBXCHILD's deletion writes
  there, with its PID, its status, `RMS$_FNF`, and its owner, MBXTEST.
  Then it starts **MBXPARENT**, a detached process running MBXTEST.EXE
  again, which starts its own subprocess, **MBXORPHAN**, and exits.
  MBXPARENT's termination message has no owner, and MBXORPHAN is gone
  with it. A permanent mailbox outlives its channel until `$DELMBX`.
- **GBLTEST** checks global sections: it makes a global page file section
  of two pages with `$CRMPSC`, maps it twice more with `$CRMPSC` and
  `$MGBLSC`, and reads in each mapping what it wrote in another; `$MGBLSC`
  takes or refuses an ident as its match control says. It starts
  **GBLCHILD**, the same image in a process of UIC `[200,1]` with no
  privileges, which can't see GBLTEST's group's section, writes in a
  system one, can map one the world may only read but not writable, and
  can't make one; GBLTEST reads what it wrote once its termination
  message comes. The section goes when its last mapping is deleted; a
  permanent one only after `$DGBLSC` too.
- **PTDTEST** checks pseudo terminals: it makes one with `PTD$CREATE`,
  `FTAn:`, and assigns it by the name `$GETDVI` gives, as any process
  would. A read with a prompt there writes the prompt, which `PTD$READW`
  reads; `PTD$WRITE` types `hi` and a carriage return, which end the
  read, and `PTD$READW` reads their echo; a write there waits until
  `PTD$READW` reads it. `$BRKTHRU` writes a message there through a
  read with `ab` typed, and the read writes its prompt and `ab` again
  after it. `PTD$DELETE` hangs the terminal up, and the read waiting
  there ends with `SS$_HANGUP`.
- **LOCKTEST** checks the lock manager, `$ENQ`, `$ENQW`, `$DEQ` and
  `$GETLKI` ([ADR-0030](adr/0030-lock-manager-in-the-executive.md)): two
  protected reads of one resource are granted together, but not an
  exclusive lock nor a conversion to one with `LCK$M_NOQUEUE`
  (`SS$_NOTQUEUED`); an exclusive request waits, and is granted, with its
  AST, once both reads are dequeued; a conversion down to null is granted
  at once (`SS$_SYNCH`); a waiting lock can't be converted nor have a
  sublock, `$GETLKI` names the lock that blocks it, and it ends with
  `SS$_ABORT` when it is dequeued. A value block an exclusive lock writes
  as it converts down is read by the next lock, and once an exclusive lock
  is dequeued with `LCK$M_INVVALBLK` the next read gets `SS$_VALNOTVALID`.
  A lock's blocking AST runs once another request waits for it, and
  dequeues it. A sublock is another resource than the same name without a
  parent, and its parent can't be dequeued before it. Then it starts
  **LOCKTEST2**, the same image, which finds itself the second because it
  can't have the lock on `LOCKTEST$ROLE` that LOCKTEST holds. Each takes
  a lock and asks for the other's, a deadlock: about ten seconds later
  (`DEADLOCK_WAIT`, [ADR-0035](adr/0035-deadlock-search-after-deadlock-wait.md))
  the lock manager's search, `LCK$SEARCH`, fails one of the two requests
  with `SS$_DEADLOCK`, and that process gives up its lock. LOCKTEST2 then
  takes `LOCKTEST$S`, wakes LOCKTEST, waits for `LOCKTEST$R`, which
  LOCKTEST holds, and exits as soon as it gets it, still holding both. Its image rundown dequeues them,
  which grants LOCKTEST the lock on `LOCKTEST$S` it waits for, and
  LOCKTEST prints `LOCKTEST: ok`.
- **FSTEST1** and **FSTEST2** each count the files in `SYS$SYSTEM:` 50
  times with `$PARSE` and `$SEARCH`, at once. The file system runs
  holding a lock of its own, `FIL$LOCK`, below `IPL$_SYNCH`
  ([ADR-0020](adr/0020-file-system-lock-below-ipl-synch.md)), so the
  timer takes the CPU from one while it holds the lock, and the other
  waits for it. Each prints `FSTEST: ok` if every count was the first's.
- **CHFTEST** checks condition handlers: a `LIB$SIGNAL` that its own
  handler continues, one an inner routine's handler passes on to it, an
  access violation that `LIB$SIG_TO_RET` turns into the status of the
  routine that took it, and a `BPT` whose handler sends it on at another
  PC with its registers as they were. With no handler, a warning is
  printed and the program goes on; `LIB$STOP` prints its condition and
  exits, through the exit handler that prints `CHFTEST: ok`.
- **QUOTATEST** checks the buffered I/O quotas: it queues reads on a
  mailbox, which wait for a message, until it has none of its `BIOLM`
  left, each read taking one of `BIOCNT` and its IRP's bytes of
  `BYTCNT`, as `$GETJPI` shows. With resource wait off (`$SETRWM`) one
  more read fails with `SS$_EXQUOTA`; with it on, one more waits until a
  timer's AST cancels the others, which gives their quota back. It prints
  `QUOTATEST: ok` once deassigning the channel has given back the last.
- STARTUP prints `STARTUP: done` and exits, and DCL prompts again while
  the others finish.

`RUN PROTTEST` starts **PROTTEST**, which makes **PROTCHILD**, the same
image in a process of UIC `[200,1]`, with no privileges, and waits until
it ends. PROTCHILD reads `DKB0:[000000]DATA.TXT`, then tries to give it
another owner, delete it and make a file next to it, which it may not:
PROTTEST says so, or returns the status that stopped PROTCHILD,
`%RMS-E-PRV` for a file whose world may not read it.

`RUN SPIN` starts **SPIN**, which computes forever with a value in each
register, and checks them, so CTRL/Y and `CONTINUE` can be tried on it.
`RUN CTRLC` starts **CTRLC**, which asks for a CTRL/C AST and waits for a
line; CTRL/C runs its AST, which cancels the read with `$CANCEL`.

`cargo test -p boot` boots the system, logs in, as `NOBODY`, then as
`SYSTEM` with a wrong password, both of which fail, then as `SYSTEM`,
whose login reports the one failure, types `RUN STARTUP`, `RUN SNOOP`, a bad
command, `DIR [SYSEXE]P%NG`, `DIR SYS$SHARE:`, `TYPE WELCOME.TXT`,
`@DCLTEST 3 "Two words"`, whose procedure, `[SYSMGR]DCLTEST.COM`, counts
in a loop, checks expressions and calls itself, `@DCLTEST FAIL`, which
stops at a `TYPE` that fails, `SHOW SYMBOL $STATUS`, a symbol it writes,
and `HOME`, SYLOGIN's, then an `EDIT WELCOME.TXT`
session, and `DEFINE`, `SHOW
LOGICAL` and `DEASSIGN` of a logical name, `SHOW LOGICAL` alone, and
`SET DEFAULT` and `SHOW DEFAULT` with a `DIR` between, then `[-]` and a
`DIR [.SYSMGR]`, at the prompt, then goes to `MDA0:`, which
`SYSTARTUP_VMS.COM` mounted,
copies a file to it with `COPY/LOG`, which prompts for the two files,
changes it in `EDIT`'s line and keypad modes into
a second version, lists them,
deletes them, the first with `DELETE/LOG`, and lists again, runs
**CLITEST**, which checks the command parser on commands of its own,
a `DIR/BRIEFLY`, a qualifier `DIRECTORY` doesn't have, `HELP SHOW`, and
a `DEFINE/SYSTEM` it looks up with `SHOW LOGICAL/SYSTEM`, makes a logical
name table with `CREATE/NAME_TABLE`, a name in it that `SHOW LOGICAL`
finds once the process has an `LNM$FILE_DEV` of its own, in its
`LNM$PROCESS_DIRECTORY`, which `SHOW LOGICAL/TABLE` lists, and a table
under it that goes when another replaces it, then
initializes, with `/PROTECTION`, and mounts `DKB0:`, made afresh, which `SYSTARTUP_VMS.COM`
couldn't mount at boot, copies a file there and lists
it, makes `[SUB.DEEP]` there with `CREATE/DIRECTORY`, copies a file
into it and types another with `CREATE`, runs `SHOW DEVICES`, tries to dismount `DKA0:`, dismounts
`DKB0:`, fails to list it, mounts it again, with the logical name `WORK`, lists
`[SUB.DEEP]`, fails to delete `SUB.DIR`, runs `SHOW PROCESS` and `SHOW SYSTEM`, then runs
CTRLC and types CTRL/C, types CTRL/Y at the prompt, stops SPIN, with a
CTRL/C, and SLEEPER with CTRL/Y and continues them, and EDIT while it
reads, so that DCL reads the next commands, takes CMKRNL and SYSNAM away
with `SET PROCESS/PRIVILEGES`, so that `SHOW LOGICAL` says
`%SYSTEM-F-NOPRIV`, while `DEFINE/SYSTEM` still works, as SYSTEM's UIC
group is a system one, gives them back, runs PROTTEST, which can't read
`DATA.TXT` on `DKB0:` until `SET PROTECTION=(W:R)`, then can as its
owner after `SET FILE/OWNER_UIC=[200,1]` and `SET PROTECTION=(W)`, with
`DIRECTORY/OWNER/PROTECTION` after each, and can't once `DKB0:` is
mounted `/PROTECTION=(W)`; with `SET SECURITY` it gives `DATA.TXT` ACEs
that let PROTTEST read it, or not, which `SHOW SECURITY` shows, and a
directory default ACEs its new files get; mounts `DKB0:` without
`/SYSTEM`, for its job alone, which a process of another job can't list,
dismount or mount, and with `/NOWRITE` and `/OVERRIDE=IDENTIFICATION`,
and sees another job's private mount go with its process; adds JOE, of UIC `[200,1]`, with `MCR
AUTHORIZE`, shows him and lists the users in `SYSUAF.LIS`, adds the
identifier `PAYROLL` and grants it to him, changes
SYSTEM's password with `SET PASSWORD`, logs out and in as JOE, who has
his UIC, privileges, default directory and `PAYROLL`, which lets him read
a file the world may not, and may not run AUTHORIZE or
OPCCRASH, `DEFINE/SYSTEM` or make a table in `LNM$SYSTEM_DIRECTORY`, but
may in his own directory, logs out again and in as SYSTEM with the new password, and
looks for the success lines in `out/serial.log`. The passwords typed
are nowhere in it. Once QEMU is gone, `ods-image` checks
the data disk's volume and finds the files on it, `[SUB.DEEP]`'s too. The test runs in
named phases, `boot`, `system disk`, `ramdisk` and so on, each with its own
deadline, and one that runs out fails with the phase it stalled at, what it
was waiting for and the last line the console printed.

`cargo test -p boot --test network` boots one system on QEMU's user
network, sets and shows the interface, runs TCPTEST against a server and
a client of its own and with a UDP datagram both ways, and logs in to the
system itself, as SYSTEM, with `SET HOST`.
Then it boots two, on one QEMU socket network, saves an address on each
and applies it with `TCPIP START COMMUNICATION`, adds JOE on one with
`AUTHORIZE`, and logs in from the other as JOE, with his UIC and
privileges.
