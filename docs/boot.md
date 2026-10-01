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
four files into memory: the shim, the seL4 kernel, the root task and
`sys.vol`. `sys.vol` is the boot volume, a small disk image that holds the
VMS executable files. Limine then jumps to the shim.

## 2. The shim

The shim ([shim/src/main.c](../shim/src/main.c), `shim_main`) is a small C
program. Its job is to put the machine in the state seL4 expects at startup
([shim/README.md](../shim/README.md) has the details).

- It looks up the serial port's address in the hardware description (the
  DTB) so it can print. That's the `vaxpunk shim:` line.
- It copies the kernel, then the DTB, then the root task into physical
  memory, one after another. The boot volume goes right after the root task,
  so seL4 will treat it as part of the root task's program.
- It sets up the MMU and jumps into seL4.

## 3. seL4

seL4, the microkernel, takes over the CPU and memory. It creates one
program, the root task, and gives it every permission ("capability") in the
system. The boot volume is mapped at the end of the root task's memory.

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
- `start_exec` finds `EXEC.EXE` on the boot volume and loads it. It also
  creates the restart parameter block (RPB), a page describing memory, the
  volume and the boot time. It then starts EXEC in kernel mode at IPL 31,
  with R11 pointing at the RPB.
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
- lowers IPL to 0 and creates the console's process, SYSTEM, which runs
  `DCL.EXE`
- then becomes the swapper (process 1). The swapper cleans up deleted
  processes and sleeps the rest of the time.

## 6. SYSTEM and DCL

DCL ([roottask/sysexe/dcl.mar](../roottask/sysexe/dcl.mar)) is the command
interpreter. It is linked high in P1, which tells the executive it is one
([ADR-0006](adr/0006-cli-in-p1-runs-images-in-its-process.md)):

- When SYSTEM starts (`EXE$PROCSTRT`), the executive makes its stacks,
  loads `DCL.EXE` into P1 and calls it in supervisor mode.
- DCL opens a channel to the console, `OPA0:`, prints the `$` prompt and
  waits for a line. That's where the boot ends: the CPU idles, taking
  clock ticks, until you type something.
- `RUN image` loads the image into the same process's P0 (`$IMGACT`) and
  runs it in user mode. When the image exits, the executive throws away
  its pages and the channels it opened, and calls DCL again with the
  exit status. DCL prints a message if the status is an error, then the
  prompt again.
- `DIRECTORY` (`DIR`) runs `DIRECTORY.EXE`, which lists the files on the
  boot volume. `HELP` lists the commands, and `LOGOUT` deletes SYSTEM.

There's no login or disk driver yet, and no CTRL/Y: a program that never
exits keeps the console.

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

`just check` boots the system, types `RUN STARTUP`, `RUN SNOOP`, a bad
command and `DIR P%NG` at the prompt, and looks for the success lines in
`out/serial.log`.
