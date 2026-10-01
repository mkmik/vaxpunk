# ADR-0003 — Processes are threads that take turns on one VMS CPU

Oct 1, 2026 · @Marko Mikulicic

Accepted. Each executive process runs in a seL4 thread of its own, in the
executive's address space, but only one of them runs at a time: the PAL
gives the one CPU the executive sees to the thread of the process the
executive's scheduler switches to with `SWPCTX`. The executive synchronizes
as uniprocessor VMS did, with IPL.

## Context

[ADR-0002](0002-root-task-is-the-pal.md) made the executive a task that
calls the PAL with privileged instructions. The executive now needs
processes: images loaded from a volume, started, scheduled and deleted the
way a VMS process is, in kernel mode for now. Supervisor and user mode will
follow, each a task with an address space of its own.

VMS's executive is written for its CPU: one thread of control per CPU,
whose IPL decides what may interrupt it. Raising IPL to `IPL$_SYNCH` keeps
the scheduler's database consistent on a uniprocessor; on a multiprocessor
the same code also takes a spinlock that raises IPL. The scheduler runs at
`IPL$_RESCHED`, through a software interrupt, and switches processes with
`SVPCTX` and `LDPCTX` on a VAX, `SWPCTX` on Alpha. Executive code is full
of the assumption that nothing else runs while it holds IPL.

seL4 schedules threads by priority and budget and can preempt any of them
at any instruction. It can't mask interrupts for a thread.

## Decision

1. **A process is a thread.** The PAL makes one, with a TCB and a scheduling
   context, the first time the executive switches to the process's hardware
   PCB, and deletes it on `DELCTX`. All of them share the executive's
   address space and run in kernel mode.
2. **One CPU.** Only the current process's thread is runnable. Every other
   one waits in a PAL call, `SWPCTX`, or hasn't started. `SWPCTX` keeps the
   caller waiting and lets the new process's thread run: it returns from its
   own `SWPCTX`, or, the first time, starts as `REI` from the frame at its
   kernel stack pointer. seL4 never has two executive threads to choose
   between.
3. **The executive schedules.** Its scheduler, in MACRO-32, keeps VMS's
   state queues and priorities and picks the process that runs. seL4's
   scheduler only runs the PAL above whichever executive thread holds the
   CPU.
4. **IPL is the synchronization.** Since one thread runs and interrupts
   reach it only when the PAL delivers them, which it does when IPL allows,
   IPL protects data exactly as on a uniprocessor VAX. `IPL$_SYNCH`
   protects the scheduler's database, the PFN database and pool;
   `SOFTINT #IPL$_RESCHED` asks for a reschedule, which happens when IPL
   drops below 3.
5. **Interrupts are delivered at PAL calls for now.** The only interrupts
   are software interrupts, requested with `MTPR #PR$_SIRR`, and the PAL
   delivers each when a call lowers IPL below it: `MTPR #PR$_IPL`, `REI`,
   or the start of a new process.

## Alternatives considered

| Option | Why not |
| --- | --- |
| seL4 schedules the processes: all their threads runnable at their priorities | Executive code could be preempted anywhere by another thread running executive code. IPL would protect nothing, and every structure would need SMP-style locks from the start, in code written for IPL. |
| One thread for all processes; `SWPCTX` saves and loads its registers | Literal Alpha, and one TCB fewer per process. But the user and supervisor mode tasks a process will get need a kernel-mode thread to fault to and wait in, and each switch would cost the PAL a full register read and write. |
| Processes as separate tasks, each with its own address space, now | That is where user mode goes. Kernel mode is shared by every process on VMS, so its code and data live in one address space. |
| Spinlocks instead of IPL, as on SMP VMS | Needed with more than one CPU. With one, VMS's own spinlock macros reduce to raising IPL; vaxpunk does the same until it runs on more cores. |

## Consequences

**What gets harder.**
- A process can't be preempted until it makes a PAL call: there is no
  clock interrupt yet. The quantum will come from MCS timeout faults, which
  stop the running thread and let the PAL deliver an interval timer
  interrupt the same way it delivers software interrupts.
- The PAL can't let a second core run executive threads without the
  executive taking spinlocks. More CPUs means `MTPR_IPIR`, spinlocks, and a
  current process per CPU.
- A process the executive deletes from outside isn't running, so it waits
  in a queue and holds nothing else; once processes hold resources across
  waits, deletion becomes a kernel AST delivered in the process's context,
  as on VMS.

**What stays easy.**
- The executive's scheduler, IPL macros and wait queues are VMS's. Code
  that synchronizes with `DSBINT #IPL$_SYNCH` is correct as written.
- Supervisor and user mode add tasks to a process without changing how
  the CPU moves between processes: the PAL hands a mode task's `CHMK` to
  its process's kernel thread.

**Follow-ups:** the interval timer and quantum end, ASTs (`IPL$_ASTDEL`
and the `ASTSR` registers), supervisor and user mode as tasks, and
spinlocks with a second CPU.
