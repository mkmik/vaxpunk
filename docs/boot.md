# What happens during boot

The boot sequence, from power on to the DCL prompt on the console, and
what the test programs you can run from there do, in plain words. It is
kept current: a PR that changes what boot does changes this file too (see
[AGENTS.md](../AGENTS.md)).

Booting goes through five programs, each starting the next one. Then the
executive starts the console's process, which runs DCL and waits for
commands.

## 1. Firmware, then Limine

QEMU's firmware finds the boot disk (`out/esp.img`) and starts Limine, a
bootloader. As [image/limine.conf](../image/limine.conf) says, Limine loads
three files into memory: the shim, the seL4 kernel and the root task.
Limine then jumps to the shim.

QEMU has a second disk, the system disk (`out/sysdisk.img`). It is a
Files-11 ODS-2 volume, the file system VMS uses, which
[roottask/build.rs](../roottask/build.rs) makes at build time: the VMS
executable files in `[SYSEXE]` and a text file in `[SYSMGR]`. The firmware
and Limine leave it alone; the root task reads it later.

## 2. The shim

The shim ([shim/src/main.c](../shim/src/main.c), `shim_main`) is a small C
program. Its job is to put the machine in the state seL4 expects at startup
([shim/README.md](../shim/README.md) has the details).

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

The root task ([roottask/src/main.c](../roottask/src/main.c), `main`) plays
the role PALcode played on an Alpha: it's the "hardware" layer underneath VMS
([ADR-0002](adr/0002-root-task-is-the-pal.md)).

- It maps the serial port and the real-time clock, and reads the clock once
  to get the boot time. Then it prints `hello from the root task` and a list
  of the free memory blocks seL4 gave it.
- It starts a clock thread that wakes up every 10 ms. Each wakeup becomes
  VMS's timer interrupt ([ADR-0004](adr/0004-interval-timer-is-a-pal-thread.md)).
  On each wakeup the PAL also checks the serial port for typed
  characters, and once the executive asks for it, raises the console
  receive interrupt when there are some.
- `disk_init` finds the disks, virtio block devices: unit 0, the system
  disk, and unit 1, the data disk, `out/datadisk.img`, and prints
  `disk 0: virtio-blk, 4096 blocks` and `disk 1: ...` for them. From then
  on the PAL can read the disks' blocks, by number, for itself and for the
  executive, and write the data disk's for the executive. It also
  notes the network device, virtio-net, if QEMU has one.
- `start_exec` reads `EXEC.EXE` from the system disk, as VMS's first
  bootstrap did: the home block, the index file, the top directory,
  `[SYSEXE]`, then the file (`f11_boot_file`). It loads it, and creates
  the restart parameter block (RPB), a page describing memory and the
  boot time. With a network device it also makes the port's 17 pages
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

The executive is the VMS kernel, written in MACRO-32.
`EXEC$START` ([roottask/exec/exec.mar](../roottask/exec/exec.mar)):

- fills in the system control block (handlers for traps, mode changes and
  software interrupts, plus the timer and the console receive interrupt)
- starts memory management (`MMG$INIT`) and lets user-mode code use the
  system service vector
- starts the scheduler (`SCH$INIT`) and the system time (`EXE$INITTIM`)
- empties the type-ahead buffer, where typed characters wait until a
  program reads them, and turns on the console receive interrupt
  (`TTY$INIT`)
- finds the port in the RPB (`NET$INIT`); its interrupt, at IPL 21, and
  the software interrupt it requests, at IPL 6, are in the SCB too
- prints `%EXEC-I-START … free pages`
- mounts the system disk, `DKA0:`: it reads the volume's home block and
  the index file's header, which says where every other file's header is
  (`FIL$MOUNT`), handing each read to the disk's driver (`DK$STARTIO`) in
  an I/O request packet, as the file system always does, and prints `%MOUNT-I-MOUNTED, VAXPUNK mounted on _DKA0:`
- defines the system's logical names for it, in `LNM$SYSTEM_TABLE`:
  `SYS$SYSDEVICE` is `DKA0:`, `SYS$DISK`, the default device, is
  `SYS$SYSDEVICE:`, `SYS$SYSTEM`, where the images are, is
  `SYS$SYSDEVICE:[SYSEXE]`, and `SYS$MANAGER`, where the system manager's
  files are, is `SYS$SYSDEVICE:[SYSMGR]`
- lowers IPL to 0 and creates the console's process, SYSTEM, which runs
  `DCL.EXE`, with the logical names `SYS$INPUT`, `SYS$OUTPUT` and
  `SYS$ERROR` standing for the console, `_OPA0:`, in its process table
- then becomes the swapper (process 1). The swapper cleans up deleted
  processes and sleeps the rest of the time.

## 6. SYSTEM and DCL

DCL ([roottask/sysexe/dcl.mar](../roottask/sysexe/dcl.mar)) is the command
interpreter. It is linked high in P1, which tells the executive it is one
([ADR-0006](adr/0006-cli-in-p1-runs-images-in-its-process.md)):

- When SYSTEM starts (`EXE$PROCSTRT`), the executive makes its stacks,
  reads `SYS$SYSTEM:DCL.EXE`, which is `DKA0:[SYSEXE]DCL.EXE`, from the
  disk (`FIL$OPENFILE`), loads it
  into P1 and calls it in supervisor mode.
- DCL opens a channel to `SYS$INPUT`, which `$ASSIGN` translates to the
  console, `OPA0:`. Its first command is `@SYS$MANAGER:SYSTARTUP_VMS`,
  which runs the command procedure `DKA0:[SYSMGR]SYSTARTUP_VMS.COM`, the
  site's own startup, as VMS runs it once at boot: its `MOUNT DKB0:`
  mounts the data disk, whatever its label, if an `INITIALIZE DKB0:`
  wrote a volume there, at this boot or an earlier one, and prints
  `%MOUNT-I-MOUNTED, label mounted on _DKB0:`. On a blank disk it prints
  `%SYSTEM-W-NOHOMEBLK` instead, and the disk stays unmounted. The second
  is `@SYS$MANAGER:SYLOGIN`, which
  runs `DKA0:[SYSMGR]SYLOGIN.COM`, as VMS runs it
  at each login: it defines the global symbol `HOME`, a command that goes
  back to `[SYSMGR]`, and runs `TCPIP.EXE`. With a network, that sets
  the interface's address, mask and gateway as `SET CONFIGURATION
  INTERFACE` and `SET ROUTE /PERMANENT` last saved them on the data disk, and prints `%TCPIP-I-SET, BGA0: ...`, and
  creates the process `TCPIP$TELNET`, which runs `TELNETD.EXE` and waits
  on TCP port 23 for `SET HOST` from another vaxpunk. Then DCL reads a line with the `$` prompt: `$QIOW`
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
  `'name'` in a command stands for its value. `IF expression THEN
  command`, `GOTO label` and `WRITE SYS$OUTPUT` work in procedures as in
  VMS's, and `$STATUS` holds the last command's status: one that is an
  error ends the procedures, as VMS's default `ON ERROR THEN EXIT` does.
- DCL parses each command with its command tables, `DCL$TABLES`, which
  `roottask/build.rs` compiled from the verbs' definitions in
  `roottask/cld/*.cld` with vcdu, and linked into `DCL.EXE`. The parser,
  `CLI$$DCL_PARSE` in `roottask/sysexe/lib/cli.mar`, matches the verb on
  its first 4 characters, then its parameters and its qualifiers, such as
  `/LOG`. It prompts for a parameter the command needs but lacks, `_From: `,
  and reports a mistake as VMS does: `%DCL-W-IVQUAL, unrecognized
  qualifier - check validity, spelling, and placement`, then the word in
  backslashes. A verb that runs an image passes it the parse, which the
  image reads with `CLI$GET_VALUE` and `CLI$PRESENT`; DCL does the others
  itself, and reads their values and qualifiers the same way.
- `RUN image` reads the image from `SYS$SYSTEM:` and loads it into the same
  process's P0 (`$IMGACT`) and runs it in user mode. When the image
  exits, the executive throws away its pages and the channels and files
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
  `SHOW.EXE`, which asks `$GETJPI` about this process, and `$GETSYI` how
  long the system has been up, then `$GETJPI` with a wildcard for a line
  per process: its PID, name, state, priority and image. `HELP` runs
  `HELP.EXE`, which lists the commands from DCL's command tables, and
  `LOGOUT` deletes SYSTEM.
- `INITIALIZE MDA0: label` runs `INIT.EXE`, whose `$INIT_VOL` makes the
  ramdisk, `MDA0:`, 512 KB of memory, and writes an empty volume on it.
  `MOUNT MDA0: label` runs `MOUNT.EXE`, whose `$MOUNT` mounts it and prints
  `%MOUNT-I-MOUNTED, label mounted on _MDA0:`. Then `COPY` makes files
  there (`COPY WELCOME.TXT MDA0:[000000]`, from `[SYSMGR]`, the default), `DELETE` deletes them
  (`DELETE MDA0:[000000]WELCOME.TXT;1`), and `DIRECTORY`, `TYPE` and `RUN`
  read them as they do the system disk's.
- `INITIALIZE DKB0: label` and `MOUNT DKB0: label` do the same on the
  data disk, `DKB0:`, a disk image on the host which the PAL writes with
  `WRITELBLK` ([ADR-0012](adr/0012-data-disk-writable-files-11.md)). What
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
- `DISMOUNT DKB0:` runs `DISMOUNT.EXE`, whose `$DISMOU` makes the file
  system forget the volume, unless a process has a file on it open, so
  it can be mounted again, or initialized afresh. The system disk can't
  be dismounted (`%SYSTEM-F-DEVACTIVE`).
- `SHOW DEVICES` (`SHO DEV`) lists the disks, mounted or not, with the
  errors the disk's driver counted in its UCB, and their volumes' labels
  and free blocks, then the console, `OPA0:`, as VMS
  does: it finds them with the `$DEVICE_SCAN` system service and asks
  `$GETDVIW` about each, which counts the free blocks in the volume's
  storage bitmap (`FIL$FREEBLOCKS`). `SHOW DEVICES DK` lists only the
  devices whose names start with `DK`.
- A command that fails prints VMS's message for the status, when DCL
  knows it: `DIR DKB0:` looks in `DKB0:[SYSMGR]`, the default directory
  on that disk, and prints `%RMS-E-DNF, directory not found`.
  When the swapper deletes what SYSTEM left, it prints `%EXEC-I-LOGOUT`
  and halts. The root task prints `%PAL-I-POWEROFF`, and `run-qemu.sh`'s
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

- `SET INTERFACE address mask`, `SET ROUTE /DEFAULT /GATEWAY=address`
  and `SHOW INTERFACE` run `TCPIP.EXE`, which senses and sets the
  interface's address, mask and gateway with `$QIOW` on `BGA0:`, the
  network's port driver. `SET CONFIGURATION INTERFACE address mask` and
  `SET ROUTE /DEFAULT /GATEWAY=address /PERMANENT` save them on the data
  disk for the next boot instead.
  `SET HOST address` runs `RTPAD.EXE`, which connects to port 23 there:
  the other side's `TELNETD` creates a process named after the
  connection's unit, `_BG02:`, running DCL with the connection as its
  input and output, and RTPAD passes lines both ways until `LOGOUT` there
  prints `%REM-S-END` here. `RUN TCPTEST` connects to a server on the
  host and accepts a connection from it.

The system disk is read only, the ramdisk is gone when the system stops,
and there's no login yet.

## 7. STARTUP and the test processes

`RUN STARTUP` at the prompt starts the tests
([roottask/sysexe/startup.mar](../roottask/sysexe/startup.mar)). STARTUP
runs inside SYSTEM; the programs it starts are processes of their own,
and each one in [roottask/sysexe/](../roottask/sysexe/) tests one
executive feature:

- STARTUP allocates 4 pages, writes them, checks what it reads back and
  frees them.
- **SLEEPER** has a higher priority than STARTUP, so it runs before STARTUP
  continues. STARTUP deletes it later.
- **PING/PONG** take turns sending signals to each other through shared
  event flags. PONG then sets a third flag of theirs, which STARTUP waits
  for.
- **SVCTEST** checks the status that each system service returns, and
  reads the system disk's home block by its LBN with `$QIOW`. It starts a
  second SLEEPER, NAPPER, and makes it exit with `$FORCEX` before its
  image runs (status `217C`). It ends by `$FORCEX`ing itself, so that
  its exit handler, declared with `$DCLEXH`, is what prints `SVCTEST: ok`.
  It also starts:
  - **SNOOP**, which reads kernel memory and should die with an access
    violation (status `0C`)
  - **USURP**, which runs a privileged instruction in user mode and should
    die with a reserved-instruction error (status `43C`)
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
  `$CREMBX` and from `$ASSIGN` by that name, and starts **MBXCHILD**, whose
  image isn't there, with the mailbox's unit for its termination mailbox:
  it reads the message MBXCHILD's deletion writes there, with its PID and
  its status, `RMS$_FNF`. A permanent mailbox outlives its channel until
  `$DELMBX`.
- **FSTEST1** and **FSTEST2** each count the files in `SYS$SYSTEM:` 50
  times with `$PARSE` and `$SEARCH`, at once. The file system runs
  holding a lock of its own, `FIL$LOCK`, below `IPL$_SYNCH`
  ([ADR-0020](adr/0020-file-system-lock-below-ipl-synch.md)), so the
  timer takes the CPU from one while it holds the lock, and the other
  waits for it. Each prints `FSTEST: ok` if every count was the first's.
- STARTUP prints `STARTUP: done` and exits, and DCL prompts again while
  the others finish.

`RUN SPIN` starts **SPIN**, which computes forever with a value in each
register, and checks them, so CTRL/Y and `CONTINUE` can be tried on it.
`RUN CTRLC` starts **CTRLC**, which asks for a CTRL/C AST and waits for a
line; CTRL/C runs its AST, which cancels the read with `$CANCEL`.

`cargo test -p boot` boots the system, types `RUN STARTUP`, `RUN SNOOP`, a bad
command, `DIR [SYSEXE]P%NG`, `TYPE WELCOME.TXT`,
`@DCLTEST 3 "Two words"`, whose procedure, `[SYSMGR]DCLTEST.COM`, counts
in a loop, checks expressions and calls itself, `@DCLTEST FAIL`, which
stops at a `TYPE` that fails, `SHOW SYMBOL $STATUS`, a symbol it writes,
and `HOME`, SYLOGIN's, then an `EDIT WELCOME.TXT`
session, and `DEFINE`, `SHOW
LOGICAL` and `DEASSIGN` of a logical name, `SHOW LOGICAL` alone, and
`SET DEFAULT` and `SHOW DEFAULT` with a `DIR` between, then `[-]` and a
`DIR [.SYSMGR]`, at the prompt, then initializes and mounts `MDA0:`,
copies a file to it with `COPY/LOG`, which prompts for the two files,
changes it in `EDIT`'s line and keypad modes into
a second version, lists them,
deletes them, the first with `DELETE/LOG`, and lists again, runs
**CLITEST**, which checks the command parser on commands of its own,
a `DIR/BRIEFLY`, a qualifier `DIRECTORY` doesn't have, `HELP SHOW`, and
a `DEFINE/SYSTEM` it looks up with `SHOW LOGICAL/SYSTEM`, then
initializes and mounts `DKB0:`, made afresh, which `SYSTARTUP_VMS.COM`
couldn't mount at boot, copies a file there and lists
it, makes `[SUB.DEEP]` there with `CREATE/DIRECTORY` and copies a file
into it, runs `SHOW DEVICES`, tries to dismount `DKA0:`, dismounts
`DKB0:`, fails to list it, mounts it again, without a label, lists
`[SUB.DEEP]`, fails to delete `SUB.DIR`, runs `SHOW PROCESS` and `SHOW SYSTEM`, then runs
CTRLC and types CTRL/C, types CTRL/Y at the prompt, stops SPIN, with a
CTRL/C, and SLEEPER with CTRL/Y and continues them, and EDIT while it
reads, so that DCL reads the next commands, and looks for the
success lines in `out/serial.log`. Once QEMU is gone, `ods-image` checks
the data disk's volume and finds the files on it, `[SUB.DEEP]`'s too.

`cargo test -p boot --test network` boots one system on QEMU's user
network, sets and shows the interface, runs TCPTEST against a server and
a client of its own, and logs in to the system itself with `SET HOST`.
Then it boots two, on one QEMU socket network, each with a data disk it
made holding saved settings, and logs in from one to the other.
