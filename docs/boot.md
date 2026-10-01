# What happens during boot

The boot sequence, from power on to the last test process, in plain words.
It is kept current: a PR that changes what boot does changes this file too
(see [AGENTS.md](../AGENTS.md)).

Booting goes through five programs, each starting the next one. Then the
executive starts its processes.

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
  software interrupts, plus the timer)
- starts memory management (`MMG$INIT`) and lets user-mode code use the
  system service vector
- starts the scheduler (`SCH$INIT`) and the system time (`EXE$INITTIM`)
- prints `%EXEC-I-START … free pages`
- lowers IPL to 0 and creates the STARTUP process
- then becomes the swapper (process 1). The swapper cleans up deleted
  processes and sleeps the rest of the time.

## 6. STARTUP and the test processes

STARTUP ([roottask/sysexe/startup.mar](../roottask/sysexe/startup.mar)) is
the first real process. Each process in
[roottask/sysexe/](../roottask/sysexe/) is a test of one executive feature:

- STARTUP allocates 4 pages, writes them, checks what it reads back and
  frees them.
- **SLEEPER** has a higher priority than STARTUP, so it runs before STARTUP
  continues. STARTUP deletes it later.
- **PING/PONG** take turns sending signals to each other through shared
  event flags. PONG wakes STARTUP when they're done.
- **SVCTEST** checks the status that each system service returns. It also
  starts:
  - **SNOOP**, which reads kernel memory and should die with an access
    violation (status `0C`)
  - **USURP**, which runs a privileged instruction in user mode and should
    die with a reserved-instruction error (status `43C`)
- **HOG** spins without ever waiting. **NUDGE** can only run if the timer
  interrupt takes the CPU away from HOG, so it tests preemption.
- **TIMETEST** checks reading the time, timers and scheduled wakeups.
- STARTUP prints `STARTUP: done` and exits.

After that only the swapper is left. It sleeps, and the PAL keeps delivering
clock ticks to an idle CPU. `just check` boots the system and looks for those
success lines in `out/serial.log`.

There's no login, shell or disk driver yet: the boot ends once those test
processes finish.
