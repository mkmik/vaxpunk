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
- `disk_init` finds the system disk, a virtio block device, and prints
  `disk: virtio-blk, 4096 blocks`. From then on the PAL can read the
  disk's blocks, by number, for itself and for the executive.
- `start_exec` reads `EXEC.EXE` from the system disk, as VMS's first
  bootstrap did: the home block, the index file, the top directory,
  `[SYSEXE]`, then the file (`f11_boot_file`). It loads it, and creates
  the restart parameter block (RPB), a page describing memory and the
  boot time. It then starts EXEC in kernel mode at IPL 31, with R11
  pointing at the RPB.
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
- prints `%EXEC-I-START … free pages`
- mounts the system disk, `DKA0:`: it reads the volume's home block and
  the index file's header, which says where every other file's header is
  (`FIL$MOUNT`), and prints `%MOUNT-I-MOUNTED, VAXPUNK mounted on _DKA0:`
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
  reads `DKA0:[SYSEXE]DCL.EXE` from the disk (`FIL$OPENFILE`), loads it
  into P1 and calls it in supervisor mode.
- DCL opens a channel to `SYS$INPUT`, which `$ASSIGN` translates to the
  console, `OPA0:`, prints the `$` prompt and waits for a line. That's where the boot ends: the CPU idles, taking
  clock ticks, until you type something.
- `RUN image` reads the image from `[SYSEXE]` and loads it into the same
  process's P0 (`$IMGACT`) and runs it in user mode. When the image
  exits, the executive throws away its pages and the channels and files
  it opened, and calls DCL again with the exit status. DCL prints a
  message if the status is an error, then the prompt again.
- `DIRECTORY` (`DIR`) runs `DIRECTORY.EXE`, which lists files with RMS's
  `$PARSE` and `$SEARCH`: those in the default directory, `[SYSMGR]` until
  `SET DEFAULT` changes it (`$SETDDIR`; `SHOW DEFAULT` shows it), or
  the ones it is given, `DIR [SYSEXE]`, `DIR [000000]`. `TYPE file` runs
  `TYPE.EXE`, which reads the file with `$OPEN` and `$GET` and writes it
  on the console. `EDIT file` runs `EDIT.EXE`, EDT's line mode: it reads
  the file the same way and, at its `*` prompt, types the lines you ask
  for, until `EXIT` or `QUIT`. It can't change them yet. `DEFINE`,
  `DEASSIGN` and `SHOW LOGICAL` make, delete
  and translate logical names (`$CRELNM`, `$DELLNM`, `$TRNLNM`), and
  `SHOW LOGICAL` alone lists them. `HELP` lists the commands, and `LOGOUT` deletes SYSTEM.
- `INITIALIZE MDA0: label` runs `INIT.EXE`, whose `$INIT_VOL` makes the
  ramdisk, `MDA0:`, 512 KB of memory, and writes an empty volume on it.
  `MOUNT MDA0: label` runs `MOUNT.EXE`, whose `$MOUNT` mounts it and prints
  `%MOUNT-I-MOUNTED, label mounted on _MDA0:`. Then `COPY` makes files
  there (`COPY WELCOME.TXT MDA0:[000000]`, from `[SYSMGR]`, the default), `DELETE` deletes them
  (`DELETE MDA0:[000000]WELCOME.TXT;1`), and `DIRECTORY`, `TYPE` and `RUN`
  read them as they do the system disk's.
  When the swapper deletes what SYSTEM left, it prints `%EXEC-I-LOGOUT`
  and halts, and the root task powers QEMU off with a semihosting
  `SYS_EXIT`.
- CTRL/Y while an image runs stops it where it is: the console prints
  `*INTERRUPT*`, and the executive calls DCL again with `SS$_CONTROLY`,
  leaving the image as it was on the kernel stack (`EXE$CTRLY`,
  [ADR-0010](adr/0010-ctrly-calls-the-cli-on-top-of-the-image.md)). DCL
  prompts. `CONTINUE` goes back to the image with `$CONTINUE`; a command
  that runs another image throws the stopped one away first.

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
- **SVCTEST** checks the status that each system service returns. It also
  starts:
  - **SNOOP**, which reads kernel memory and should die with an access
    violation (status `0C`)
  - **USURP**, which runs a privileged instruction in user mode and should
    die with a reserved-instruction error (status `43C`)
- **HOG** spins without ever waiting. **NUDGE** can only run if the timer
  interrupt takes the CPU away from HOG, so it tests preemption.
- **TIMETEST** checks reading the time, timers and scheduled wakeups.
- STARTUP prints `STARTUP: done` and exits, and DCL prompts again while
  the others finish.

`RUN SPIN` starts **SPIN**, which computes forever with a value in each
register, and checks them, so CTRL/Y and `CONTINUE` can be tried on it.

`just check` boots the system, types `RUN STARTUP`, `RUN SNOOP`, a bad
command, `DIR [SYSEXE]P%NG`, `TYPE WELCOME.TXT`, an `EDIT WELCOME.TXT`
session, and `DEFINE`, `SHOW
LOGICAL` and `DEASSIGN` of a logical name, `SHOW LOGICAL` alone, and
`SET DEFAULT` and `SHOW DEFAULT` with a `DIR` between, then `[-]` and a
`DIR [.SYSMGR]`, at the prompt, then initializes and mounts `MDA0:`,
copies a file to it, lists it, deletes it and lists it again, then
stops SPIN and SLEEPER with CTRL/Y and continues them, and looks for the
success lines in `out/serial.log`.
