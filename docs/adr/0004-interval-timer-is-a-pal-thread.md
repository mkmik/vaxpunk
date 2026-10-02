# ADR-0004 — The interval timer is a periodic thread of the PAL's

Oct 1, 2026 · @Marko Mikulicic

Accepted. A thread of the PAL's, with a scheduling context whose budget is
smaller than its period, wakes every 10 ms and signals the PAL. The PAL
turns each tick into the VAX's interval timer interrupt, at IPL 24 through
SCB vector 0xC0, and delivers it when IPL allows: at once to a running
executive thread, which it stops wherever it is, or on the way out of a PAL
call. The executive's scheduler ends quanta from it, as VMS's does.

## Context

[ADR-0003](0003-one-cpu-many-threads.md) left the executive without a clock:
a process kept the CPU until it waited, so one that computed without a
system service starved every other at its priority. It expected the quantum
to come from MCS timeout faults.

VMS keeps time with the interval timer. Its handler, `EXE$HWCLKINT`, charges
each tick to the current process's quantum and, when it is up, requests the
software timer interrupt at `IPL$_TIMER`, whose handler calls `SCH$QEND`:
round robin within a priority. The executive needs the interrupt, delivered
by IPL like any other, and on time even while the CPU is idle in `WTINT`.

seL4 owns the ARM generic timer. QEMU's `virt` machine has no other timer a
user task could drive. MCS has no sleep call, but a thread whose scheduling
context has a budget below its period sleeps until its next period when it
calls `seL4_Yield`, which gives up the rest of its budget.

## Decision

1. **A clock thread.** The PAL starts a thread in its own address space, at
   the PAL's priority, above the executive's threads, with a 1 ms budget
   every 10 ms. It loops on `seL4_Yield` and `seL4_Signal` on a notification
   bound to the PAL's TCB, so the PAL's `seL4_Recv` for PAL calls returns
   the tick too.
2. **The tick is a hardware interrupt request.** The PAL keeps one pending
   bit per IPL: software interrupts at 1-15, as before, and the interval
   timer at 24. It delivers the highest one above IPL wherever it delivered
   software interrupts, through the SCB, with the same frame.
3. **Asynchronous delivery.** The PAL outranks the executive's threads and
   serves each PAL call at once, so when a tick comes the current thread is
   running or waits in `WTINT`. A running one is suspended, gets the frame
   pushed, and resumes at the handler; one in `WTINT` returns from it.
4. **The executive is VMS's.** `EXE$HWCLKINT` counts `PCB$W_QUANT` up from
   minus `SGN$GW_QUANTUM`, 20 ticks; `EXE$SWTIMINT` calls `SCH$QEND`, which
   gives the process a new quantum and requests `IPL$_RESCHED` if another
   computable process has its priority or a higher one. Real-time
   processes, 16 and up, keep the CPU.

## Alternatives considered

| Option | Why not |
| --- | --- |
| MCS timeout faults on the executive's threads, as ADR-0003 expected | They count a thread's CPU time, not the time of day: no tick while the CPU is idle in `WTINT`, so no timer could ever wake it. Every executive thread would need a budget below its period and a timeout handler. |
| A timer device driven by the PAL | QEMU's `virt` has none free for a user task: the generic timer is seL4's, and the PL031 RTC counts seconds. Real boards have one; the clock thread is replaced by a driver there. |
| Deliver ticks only at PAL calls | Exactly the starvation this removes: a loop with no system service never makes one. |
| The PAL reschedules on its own at quantum end | The PAL would decide which process runs. Quanta, priorities and round robin are the executive's, as on VMS (ADR-0003). |

## Consequences

**What gets harder.**
- An interrupt can come between any two instructions of a running process.
  The PAL pushes the frame below both VAX SP and ARM64's `sp`, since vmacro
  moves one before the other; handlers keep every register they use,
  R0 included, as VAX interrupt handlers do.
- Code that shares something must raise IPL for it, as it should have
  already. The boot volume's programs print a line at a time at
  `IPL$_SYNCH`, and so does `$EXIT`'s report.
- The CPU never idles for good: `%PAL-I-IDLE` is gone, and `cargo test -p boot`
  stops QEMU once the programs have printed their last lines.

**What stays easy.**
- The executive's timer code is VMS's shape: `EXE$HWCLKINT`, `EXE$SWTIMINT`,
  `SCH$QEND`. The system time, the timer queue (`$SETIMR`, `$SCHDWK`) and
  priority decay go in the same places.
- The tick period, `TICK_US` in the PAL, and the quantum,
  `SGN$GW_QUANTUM`, are one constant each.

**Follow-ups:** `EXE$GQ_SYSTIME` and `$GETTIM`, the timer queue, priority
boosts and decay, and a timer driver in the PAL on hardware that has one.
