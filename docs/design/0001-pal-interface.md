# DESIGN-0001 — PAL interface

Oct 1, 2026 · @Marko Mikulicic

The interface between the vaxpunk executive and the PAL below it:
- how the executive calls the PAL;
- what each call does;
- what the PAL sets up before the executive runs;
- how the PAL delivers interrupts and exceptions, and runs processes;
- the interval timer.

It builds on [ADR-0001](../adr/0001-pal-interface-vms-vocabulary.md), which
sets the vocabulary, [ADR-0002](../adr/0002-root-task-is-the-pal.md), which
makes the root task the PAL and the executive a task calling it with
privileged instructions, [ADR-0003](../adr/0003-one-cpu-many-threads.md),
which makes processes threads that take turns on one CPU,
[ADR-0004](../adr/0004-interval-timer-is-a-pal-thread.md), which makes the
interval timer a thread of the PAL's, and
[ADR-0005](../adr/0005-access-modes-are-threads.md), which gives each access
mode of a process a thread and an address space of its own, and
[ADR-0007](../adr/0007-system-disk-files-11-and-rms.md), which has the PAL
drive the system disk, and
[ADR-0011](../adr/0011-asts-on-the-kernel-stack.md), which has it request
AST delivery from bits in the HWPCB.
[DESIGN-0002](0002-executive-processes.md) covers what the executive does
with it, and [DESIGN-0003](0003-tcpip-port.md) the TCP/IP component the
PAL starts beside it and the port they share.

## The vocabulary rule

The PAL interface speaks only VMS: PCBs, PTEs, IPL, ASTs, access modes, the
system control block. It never speaks of capabilities, endpoints or
scheduling contexts. Whatever seL4 needs is translated inside the PAL. The
test for any new PAL call: could its signature have appeared in a DEC
internals manual? If it names a seL4 concept, it belongs inside the PAL, not
in its interface.

## What the PAL sets up

The root task (`pal/src/main.c`) reads `[SYSEXE]EXEC.EXE` from the
system disk (*The disks*) and starts it, in S0, the system space
every process shares (*Memory*):

| Address | What | Protection |
| --- | --- | --- |
| `0x40010000` up | `EXEC.EXE`'s sections, at their link addresses | theirs |
| `0x4FF00000`-`0x4FF11000` | the port's 17 pages, with a network device only ([DESIGN-0003](0003-tcpip-port.md)) | kernel write |
| `0x4FFF0000` | the restart parameter block, one page | kernel write |
| `0x5FFEC000`-`0x5FFF0000` | the boot stack, 16 KB | kernel write |

Every page is a PFN of the executive's (*Memory*), and the PAL uses PFNs
from 0 up for these. Nothing else is mapped. The boot context starts with:

- **Registers.** PC is the transfer address. `sp` and x18 (VAX SP) point at
  the stack top. R11 (x28) points at the RPB, as the VAX's VMB passed it.
  Everything else is 0: x9, the argument count, as for a `CALLS #0`, and
  the return address, x30. A `RET` from the transfer routine therefore
  faults at 0.
- **Processor state.** Kernel mode, the previous mode kernel too, IPL 31,
  as a VAX starts. No SCB, no software interrupts pending. The clock is already ticking: its interrupt
  waits for IPL to drop below 24.
- **Process context.** Its HWPCB is the RPB's (`RPB$Q_HWPCB`).
- **Scheduling.** Its own scheduling context, with the root task's budget
  and period, and a priority two below the root task's, one below the
  TCP/IP component's.

### The restart parameter block

`$RPBDEF` in `crosstools/vtools/lib/lib.mlb`, longwords:

| Offset | Field | Holds |
| --- | --- | --- |
| 0 | `RPB$L_BASE` | the RPB's address |
| 4 | `RPB$L_PFNCNT` | how many PFNs the executive has: 0 to `PFNCNT` - 1 |
| 8 | `RPB$L_FREEPFN` | the PFNs below this one hold what the PAL set up |
| 12 | `RPB$L_BOOTTIME` | seconds since 1970 at boot, from QEMU virt's PL031 RTC, which the PAL maps after the UART and reads once |
| 16 | `RPB$L_PORT` | port 0's pages, `0x4FF00000`, or 0 without a network device |
| 64 | `RPB$Q_HWPCB` | the boot context's HWPCB, 128 bytes |

### The disks

Unit 0, the system disk, is `sysdisk.img`, a Files-11 ODS-2 volume
labelled `VAXPUNK`, which `vms/build.rs` writes with `ods-image`:
the images in `[SYSEXE]`, SYSTEM's files in `[SYSMGR]`. QEMU attaches it
read only as a virtio-blk device on one of QEMU virt's 32 virtio-mmio
transports, from `0x0a000000`, in modern (virtio 1) mode:
`scripts/run-qemu.sh` sets `virtio-mmio.force-legacy=false`. Unit 1, the
data disk, is `datadisk.img`, another, read-write, which the script makes,
4,096 blocks of zeros, if it isn't there ([ADR-0012](../adr/0012-data-disk-writable-files-11.md)).
QEMU gives the first `-device` the highest transport, so the PAL numbers
the block devices from there down.

The PAL maps the transports from their device untyped, finds the block
devices among them and sets up one queue of four descriptors for each.
A disk's queue, a request's header and status share one page of the
PAL's, and the data of both goes through another, eight blocks at a time;
seL4 tells the PAL their physical addresses. It does a request at a time
and polls the used ring until the device is done. ponytail: polled and synchronous; the device's
interrupt, through seL4's IRQ handler, when I/O is asynchronous.

To boot, the PAL reads Files-11 as VMS's VMB did, enough to find the
executive: the home block at LBN 1, the index file's header from it, the
MFD's (file 4), `SYSEXE.DIR` in the MFD, `EXEC.EXE` in that, then the
file, through the map in its header, up to its end of file. The executive
reads and writes the disks with `READLBLK` and `WRITELBLK` (*Function codes*).

## Calling the PAL

The executive calls the PAL with `svc #0`:

| Register | In | Out |
| --- | --- | --- |
| x7 | function code, in bits 15:0; `CHMx`'s code in 31:16 | unchanged |
| x0 | a0, the first argument | v0, the result, if the call has one |
| x1-x5 | a1-a5 | unchanged |
| x6 | passed, not used | unchanged |
| x8-x29 | not seen by the PAL | unchanged |
| x30, `sp`, NZCV | seen, NZCV in the SPSR that `RD_PS` returns | unchanged |

Execution resumes after the `svc`, unless the PAL delivers an interrupt on
the way (*Interrupts and exceptions*), or the call is `REI`, a `CHMx` or
`SWPCTX`; `HALT` doesn't resume, and `WTINT` waits for an interrupt.

- **Registers.** Alpha passed a0-a5 in R16-R21 and v0 in R0. vaxpunk uses
  x0-x5 because seL4 passes the PAL only x0-x7. A call that leaves x0 alone
  returns it unchanged.
- **How it travels.** seL4 reads its syscall number from x7 and ignores the
  `svc` immediate. Every PAL code is non-negative, and seL4's own numbers
  are negative, so a PAL call reaches the PAL as an unknown syscall fault.
  - The PAL gets x0-x7, the PC of the `svc`, SP, LR and SPSR.
  - It replies with x0-x7 and the PC to resume at: the `svc`'s address
    plus 4. seL4 would otherwise run the `svc` again.
  - A call that resumes elsewhere has the PAL read and write all the
    thread's registers.
- **Privileged calls.**
  - Codes 0x00-0x7F are privileged: kernel mode only.
  - Codes 0x80-0xBF are for any mode.
  - Any other code, an unimplemented one, or a privileged call from another
    mode is a reserved instruction: an exception in an outer mode
    (*Exceptions*), and in kernel mode the PAL stops the executive.

## Memory

The executive owns physical memory as VMS does, by page frame number, and
keeps the PFN database: which PFNs are free, which hold what. A PFN is a
4 KB page the PAL makes the first time a PTE names it, from the largest RAM
untyped. ponytail: 1024 PFNs, 4 MB, as many as the root CNode's 4096 slots
leave room for.

The address space is the VAX's three regions, below 2 GB, where MACRO-32's
sign-extended longwords reach (ADR-0005):

| Region | Addresses | Whose |
| --- | --- | --- |
| P0 | `0x00000000`-`0x3FFFFFFF` | the current process's: its image, from `0x10000` |
| S0 | `0x40000000`-`0x5FFFFFFF` | the system's, the same in every process |
| P1 | `0x60000000`-`0x7FFFFFFF` | the current process's: its stacks, at the top |

Page 0 is never mapped.

The executive maps a page by writing its PTE with `WRPTE`, which takes the
place of the VAX's writing a PTE in memory and then `MTPR #PR$_TBIS`
(ADR-0001: explicit calls instead of a page table the PAL walks). A PTE in
P0 or P1 is the current process's, as the VAX's P0 and P1 page tables are.
A PTE is the VAX's longword (`$PTEDEF`), with one change:

| Bits | Field | Meaning |
| --- | --- | --- |
| 31 | `V` | valid: the page is mapped |
| 30:27 | `PROT` | the VAX protection code (`$PRTDEF`): who may read and write |
| 25 | `EXEC` | **new:** instructions may run from the page; ARM64 has the permission, the VAX had none |
| 20:0 | `PFN` | the page frame |

A valid PTE with protection `NA` maps nothing. `WRPTE` maps, unmaps or
changes protection at once, and returns the PTE the page had, so the
executive learns which PFN it freed. A PFN maps at one address at a time,
in one process if it is in P0 or P1. ponytail: a frame capability per extra
mapping, for shared pages.

What each mode sees of a page is its protection code's (`$PRTDEF`): a mode
may read a page whose code lets it or an outer mode read, and write one
whose code lets it or an outer mode write, as on the VAX. Each mode's
thread has an address space of its own (*Process contexts*) holding
exactly that:
- **Kernel mode** shares the executive's address space. It holds S0 and
  the current process's P0 and P1: on `SWPCTX` the PAL unmaps the old
  process's pages there and maps the new one's.
- **An outer mode**'s holds its process's P0 and P1 pages that mode may
  read, writable if it may write them, and the S0 pages it may read. Those
  S0 pages, such as the executive's system service vector, are the same in
  every process, and their protection is set before any outer mode runs:
  a `WRPTE` that changes an outer mode's access to an S0 page after that
  fails.

The PAL sees every PFN at an address of its own, the way Alpha PALcode read
memory physically. It reads and writes the executive's memory there to push
and pop interrupt frames and to read HWPCBs and the SCB.

## Process contexts

A process context is a thread in the executive's address space
([ADR-0003](../adr/0003-one-cpu-many-threads.md)), for kernel mode, and a
thread for each outer mode the process has entered, each in an address
space of its own ([ADR-0005](../adr/0005-access-modes-are-threads.md)). The
executive names it by its hardware PCB: a 128-byte, quadword-aligned block
(`$HWPCBDEF`) whose first four quadwords are the stack pointers of kernel,
executive, supervisor and user mode, followed by Alpha's ASTEN and ASTSR, a
byte each (*Interrupts and exceptions*). Only the current context's thread of
the current mode runs; `MFPR #PR$_PCBB` returns its HWPCB.

`SWPCTX` (a0 = the new HWPCB) gives the CPU to the context of the HWPCB:

- The caller's thread waits in its `SWPCTX`. When the CPU comes back to it,
  `SWPCTX` returns.
- A context that has run returns from its own `SWPCTX`.
- A new one, the first time the executive switches to its HWPCB, gets a
  thread whose first act is `REI` from the frame at the HWPCB's KSP. The
  executive builds that frame, with the PC to start at and its PSL.
- Either gets the HWPCB the CPU left in v0, as on Alpha.
- IPL is the CPU's: `SWPCTX` doesn't change it, but a new context's `REI`
  sets it from its frame.
- The current and previous modes are the context's: a context that has run
  is in kernel mode, in its `SWPCTX`, and gets back the previous mode it
  had.

`DELCTX` (a0 = a HWPCB) deletes the context of a process that isn't
current, and stops its threads. The PAL unmaps any pages its P0 and P1
still have; the executive frees their PFNs first, from the process itself.
The executive then frees the HWPCB and the kernel stack. A HWPCB the PAL
has no context for is fine.

## Interrupts and exceptions

The PAL delivers through the system control block, whose address the
executive sets with `MTPR #PR$_SCBB`. Its vectors are longwords at the
VAX's offsets (`$SCBDEF`): 0x10 for a reserved instruction, 0x20 for an
access violation, 0x40 + 4x for `CHMx`, 0x80 + 4n for software interrupt
level n, 0xC0 for the interval timer, 0xF8 for the console receiver,
0x100 for port 0's completion.

The PSL holds the current mode in bits 25:24, the previous mode in 23:22,
IPL in 20:16 and NZVC in 3:0. Modes are 0 kernel, 1 executive,
2 supervisor, 3 user (`$PSLDEF`). The PAL keeps the modes per context
and IPL for the CPU.

To deliver, the PAL pushes a frame and jumps to the vector, in the mode
the event goes to:
- **The same mode:** the frame goes on the current stack, 16-byte aligned
  and below both VAX SP and ARM64's `sp`.
- **An inner mode:** the PAL saves the current mode's stack pointer in the
  HWPCB, the lower of VAX SP and `sp`, and pushes the frame on the inner mode's stack, from the HWPCB, then
  moves the CPU to that mode's thread: it copies the registers and stops
  the thread it leaves.

| Offset | Holds |
| --- | --- |
| 0 | PC |
| 8 | PSL |
| 16-256 | x0-x30, 8 bytes each: R0 and R1 in x0 and x1, R2-R11 in x19-x28, AP in x12, the VAX SP in x18, FP in x29 |
| 264 | the interrupted ARM64 `sp` |

This is `$INTSTKDEF`. It holds every register of the code the event
stops, all 64 bits, as [ADR-0023](../adr/0023-calling-standard.md)
requires: a handler's longword `PUSHR` would otherwise cut the upper
halves off whatever 64-bit code it interrupted. An exception with
parameters pushes them below the frame, and its handler pops them before
`REI`. A handler may still save the VAX registers it uses, as on the VAX,
and returns with `REI`, which pops the frame, sets the modes, IPL and the
condition codes from the PSL, restores every register and the SPs from
it, and resumes at the PC. A handler that hands back a register, as a
`CHMx` handler hands back R0 and R1, writes it into the frame first. As
the VAX's, `REI` may not go to an inner mode than the current one, nor to
a previous mode inner than the new one, nor to an outer mode above IPL 0;
such a frame is a reserved instruction. Going out to another mode, it
saves the current mode's SP, past the frame, in the HWPCB. A new context
starts as `REI` from the frame at its KSP, with R0 the HWPCB the CPU left.

- **Software interrupts.** `MTPR #PR$_SIRR` requests level 1-15;
  `MFPR #PR$_SISR` shows which are pending. The PAL delivers the highest
  pending interrupt above IPL, at its level, when a call lowers IPL: `MTPR
  #PR$_IPL`, `REI`, a new context's start. A request above IPL is delivered
  on the `MTPR` that makes it.
- **ASTs.** The HWPCB's `ASTSR` byte has bit n set while an AST for mode
  n is queued, and `ASTEN` while mode n's ASTs may be delivered. The
  executive writes both, for any process; the PAL reads the current
  context's each time it may deliver, and requests software interrupt
  level 2, the VAX's AST delivery interrupt, if IPL is below 2 and a bit
  set in both is the current mode's or an inner one's, as the VAX's `REI`
  did with `ASTLVL`.
- **The interval timer.** Every 10 ms the PAL requests an interrupt at
  IPL 24, the VAX's interval timer's, through vector 0xC0. If IPL is below
  24, the PAL delivers it at once: it stops the current context wherever it
  is and resumes it at the handler, or, if it waits in `WTINT`, returns from
  that. Otherwise it stays pending, like a software interrupt, until IPL
  drops below 24. A tick while one is pending is lost, as on the VAX. The
  clock runs from boot; it has no `ICCS` to enable it. ponytail: the tick
  is a thread of the PAL's that seL4 wakes each period (ADR-0004); a timer
  driver replaces it on hardware that has a free timer.
- **The console receiver.** The VAX's console registers: `RXCS` has DONE,
  bit 7, set while a character the console received waits, and IE, bit 6,
  which the executive sets with `MTPR #PR$_RXCS` to be interrupted;
  `MFPR #PR$_RXDB` takes the character, in bits 7:0. With IE set, the PAL
  requests an interrupt at IPL 20, the VAX console's, through vector 0xF8,
  when a character waits, and delivers it as the interval timer's. The
  handler reads characters until DONE is clear. ponytail: the PAL looks at
  the UART on each tick, and on the `MTPR` that sets IE, so a character
  waits up to 10 ms; the UART's own interrupt, through seL4's IRQ handler,
  later.
- **The port.** `MTPR #n, #PR$_DOORBELL` rings port n's doorbell: the
  PAL signals the TCP/IP component's notification, and returns at once.
  When the component signals back, the PAL requests an interrupt at IPL
  21 and delivers it through vector 0x100 as the console's
  ([DESIGN-0003](0003-tcpip-port.md)). Only port 0 exists, and only with a
  network device; without one the doorbell does nothing.
- Interrupts go to kernel mode, with kernel as the previous mode, as the
  VAX's do.
- **`CHMx`**, x the mode: `CHMK` 0, `CHME` 1, `CHMS` 2, `CHMU` 3. The code in
  x7's bits 31:16, above the PAL call's, so that every other register, a
  call's arguments among them, reaches the handler as it was
  ([DESIGN-0004](0004-calling-standard.md)). The PAL delivers through the
  SCB's vector for x, at the same IPL, to mode x, or to the current mode if
  that is an inner one, with the current mode as the previous one, and
  pushes the code below the frame, in 16 bytes, as the VAX's `CHMx` pushes
  it. The handler pops it, and writes the service's status, R0, and R1 into
  the frame for `REI` to restore. Any other PAL call has bits 31:16 clear. Without a vector, or a stack in the HWPCB for
  the mode, `CHMx` is a reserved instruction.
- **`PROBER`, `PROBEW`** (a0 = an address, a1 = a length, a2 = a mode):
  v0 = 1 if the mode, or the previous mode if it is an outer one, may read
  (write) the first and the last of the a1 bytes at a0, else 0, as the
  VAX's `PROBE`. Services check the addresses their callers pass with it.
- **`WTINT`.** Returns once an interrupt IPL lets through has been
  delivered: at once if one is pending, or at the next tick.

### Exceptions

A fault or a reserved instruction in an outer mode is an exception: the
PAL delivers it through the SCB in kernel mode, at the same IPL, with the
mode that took it as the previous mode and the PC of the instruction that
took it.

| Event | Vector | Parameters, below the frame |
| --- | --- | --- |
| Undefined instruction, `brk`, a privileged or unimplemented PAL call, a `CHMx` with no vector, a bad `REI` | 0x10, reserved instruction | none |
| Page fault | 0x20, access violation | a mask, bit 2 set for a write, then the address |

### Faults and HALT

A fault or reserved instruction in kernel mode stops the executive, as
`HALT` does, and the PAL prints it, a page fault and an undefined
instruction naming the HWPCB of the process that took it:

| Event | The PAL prints |
| --- | --- |
| Page fault | `%PAL-F-ACCVIO` with the address and PC |
| Undefined instruction, `brk` (`BPT`) | `%PAL-F-OPCDEC` with the PC |
| Reserved PAL call, bad `WRPTE` | `%PAL-F-OPCDEC` with the code, PC, R0 and R1 |
| Delivery with no vector or stack | `%PAL-F-NOVEC` |
| `REI` without a frame | `%PAL-F-REI` |
| `HALT` | `%PAL-I-HALT` with the PC and R0 |

The root task then prints `root task done` and suspends itself.

## How MACRO-32 calls it

vmacro compiles privileged instructions into PAL calls, as AMACRO compiled
them into `CALL_PAL` (`crosstools/vtools/docs/macro32.md`). The executive writes VAX
source:

```
        .LIBRARY "lib.mlb"
        $PRDEF
        MTPR    #8, #PR$_IPL
```

and gets:

```
        movz    w14, #8         ; the operand
        mov     x15, x0         ; R0 aside
        mov     w0, w14         ; a0: the new IPL
        mov     x7, #15         ; MTPR_IPL
        svc     #0
        mov     x16, x0         ; v0: the old IPL, unused by MTPR
        mov     x0, x15
```

- **Registers.** R0 is x0, so vmacro keeps it in a scratch register around
  the call; x7 isn't a VAX register. Every other register, and the VAX's
  view of R0, comes back unchanged.
- **Constants.** The processor register must be a constant, as in AMACRO.
  `$PRDEF` in `crosstools/vtools/lib/lib.mlb` defines the VAX's `PR$_` names.

| VAX instruction | PAL call |
| --- | --- |
| `MTPR src, #PR$_IPL` | `MTPR_IPL`, a0 = src |
| `MFPR #PR$_IPL, dst` | `MFPR_IPL`, dst = v0 |
| `MFPR #PR$_PCBB, dst` | `MFPR_PCBB` |
| `MTPR src, #PR$_SCBB`, `MFPR #PR$_SCBB, dst` | `MTPR_SCBB`, `MFPR_SCBB` |
| `MTPR src, #PR$_SIRR` | `MTPR_SIRR` |
| `MFPR #PR$_SISR, dst` | `MFPR_SISR` |
| `MTPR src, #PR$_TXDB` | `MTPR_TXDB` |
| `MTPR src, #PR$_RXCS`, `MFPR #PR$_RXCS, dst` | `MTPR_RXCS`, `MFPR_RXCS` |
| `MFPR #PR$_RXDB, dst` | `MFPR_RXDB` |
| `MTPR src, #PR$_DOORBELL` | `MTPR_DOORBELL` |
| `CHMK #code`, `CHME`, `CHMS`, `CHMU` | `CHMK`, `CHME`, `CHMS`, `CHMU`, the code in x7's bits 31:16; R0 and R1 aren't kept |
| `PROBER mode, len, base`, `PROBEW` | `PROBER`, `PROBEW`, a0 = base, a1 = len, a2 = mode; Z set if v0 is 0, no access |
| `REI` | `REI` |
| `HALT` | `HALT` |
| `CALL_PAL #code` | any call: a0-a5 in R0-R5, v0 in R0, as on Alpha |

`CALL_PAL` reaches the calls the VAX has no instruction for: `SWPCTX`,
`WTINT`, `WRPTE`, `DELCTX`, `READLBLK`, `WRITELBLK`. `$PALDEF` in `lib.mlb` names their
codes.

## Function codes

The codes are Alpha OpenVMS PALcode's (Alpha Architecture Reference Manual,
OpenVMS software part), checked against the VMS PALcode in SIMH's Alpha
simulator. vaxpunk's own calls start at 0x40, a range Alpha OpenVMS leaves
unused.

### Implemented

| Code | Call | Origin | In | Out | What the PAL does |
| --- | --- | --- | --- | --- | --- |
| 0x00 | `HALT` | Alpha | | | stops the executive; prints its PC and R0 |
| 0x05 | `SWPCTX` | Alpha, changed | a0 = new HWPCB | v0 = the HWPCB left | switches the CPU to the HWPCB's context, making it the first time |
| 0x0E | `MFPR_IPL` | Alpha | | v0 = IPL | |
| 0x0F | `MTPR_IPL` | Alpha | a0 = new IPL | v0 = old IPL | sets the IPL, 0-31, and delivers what it lets through |
| 0x12 | `MFPR_PCBB` | Alpha | | v0 = HWPCB | the current context's HWPCB address, not a physical one |
| 0x16 | `MFPR_SCBB` | Alpha | | v0 = SCB | |
| 0x17 | `MTPR_SCBB` | Alpha | a0 = SCB | | the SCB's address, longword aligned |
| 0x18 | `MTPR_SIRR` | Alpha | a0 = level | | requests a software interrupt, 1-15 |
| 0x19 | `MFPR_SISR` | Alpha | | v0 = summary | bit n: level n pending |
| 0x3E | `WTINT` | Alpha | | v0 = 0 | waits for an interrupt; the clock's comes within 10 ms |
| 0x40 | `MTPR_TXDB` | vaxpunk | a0 = character | | writes a0's low byte on the console |
| 0x41 | `WRPTE` | vaxpunk | a0 = address, a1 = PTE | v0 = old PTE | maps, unmaps or protects a page |
| 0x42 | `DELCTX` | vaxpunk | a0 = HWPCB | | deletes a context that isn't current |
| 0x43 | `MTPR_RXCS` | vaxpunk | a0 = RXCS | | sets IE, bit 6: interrupt when a character waits |
| 0x44 | `MFPR_RXCS` | vaxpunk | | v0 = RXCS | DONE, bit 7, if a character waits, and IE |
| 0x45 | `MFPR_RXDB` | vaxpunk | | v0 = character | takes the character that waits, or 0 if none does |
| 0x46 | `READLBLK` | vaxpunk | a0 = buffer, a1 = byte count, a2 = LBN, a3 = unit | v0 = status | reads a disk's blocks from the LBN into the buffer, which kernel mode must be able to write |
| 0x47 | `WRITELBLK` | vaxpunk | a0 = buffer, a1 = byte count, a2 = LBN, a3 = unit | v0 = status | writes the buffer, which kernel mode must be able to read, to a disk's blocks from the LBN |
| 0x48 | `MTPR_DOORBELL` | vaxpunk | a0 = port | | signals the port's component; never waits |
| 0x82 | `CHME` | Alpha | the code in x7's bits 31:16 | | delivers through the SCB, to executive mode |
| 0x83 | `CHMK` | Alpha | the code in x7's bits 31:16 | | delivers through the SCB, to kernel mode |
| 0x84 | `CHMS` | Alpha | the code in x7's bits 31:16 | | delivers through the SCB, to supervisor mode |
| 0x85 | `CHMU` | Alpha | the code in x7's bits 31:16 | | delivers through the SCB, in user mode |
| 0x8F | `PROBER` | Alpha | a0 = address, a1 = length, a2 = mode | v0 = 1 if readable | checks a mode's read access |
| 0x90 | `PROBEW` | Alpha | a0 = address, a1 = length, a2 = mode | v0 = 1 if writable | checks a mode's write access |
| 0x91 | `RD_PS` | Alpha | | v0 = PSL | the current and previous modes, IPL and the condition codes |
| 0x92 | `REI` | Alpha | | | pops a frame: PC, PSL, registers, SPs |

`MTPR_TXDB` stands in for the VAX console transmit register: VMS's
`CON$PUTCHAR` writes each character with `MTPR R0, #PR$_TXDB`. Alpha had no
such register; its console output went through firmware callbacks.
The PAL writes it on the PL011 UART, which it maps from its device untyped
and drives itself, so the console needs no debug seL4. `MTPR_RXCS`,
`MFPR_RXCS` and `MFPR_RXDB` stand in for the receive registers the same way,
reading the UART (*Interrupts and exceptions*). ponytail: QEMU virt's UART
address, polled; from the DTB, with interrupts, later.

`READLBLK` reads a disk the way Alpha's console `READ` callback read the
boot disk for VMS's bootstrap, with `IO$_READLBLK`'s P1-P3 as its
arguments and the disk's unit, 0 for the system disk, 1 for the data disk
(*The disks*). It reads whole blocks and writes the first a1 bytes of
them, at most 65,535. `WRITELBLK` writes a1 bytes the same way, as
console `WRITE` did, a last block in part padded with zeros. Their status
is `SS$_NORMAL`, `SS$_ACCVIO` if a page of the buffer isn't one kernel
mode may write (read), `SS$_ILLBLKNUM` past the disk's end,
`SS$_NOSUCHDEV` with no such disk and `SS$_DRVERR` if the device fails,
as it does for a write to the system disk, which QEMU attaches read only.
The executive waits in it until the device is done.

### The Alpha calls

Each Alpha OpenVMS call, and what vaxpunk does with it:
- **kept:** same code and meaning;
- **changed:** same purpose, reshaped for seL4 as ADR-0001's table says;
- **dropped:** nothing to do under seL4, or no user.

Privileged, 0x00-0x3F:

| Code | Calls | vaxpunk |
| --- | --- | --- |
| 0x00 | `HALT` | kept, implemented |
| 0x01, 0x02 | `DRAINA`, `CFLUSH` | dropped: seL4 does cache maintenance on frames |
| 0x03, 0x04 | `LDQP`, `STQP` | dropped: the executive has no physical addresses |
| 0x05 | `SWPCTX` | changed, implemented: contexts are threads, made on first switch, deleted by `DELCTX` |
| 0x06 | `MFPR_ASN` | dropped: address space numbers are seL4's |
| 0x07, 0x08, 0x26, 0x27 | `MTPR_ASTEN`, `MTPR_ASTSR`, `MFPR_ASTEN`, `MFPR_ASTSR` | changed: bytes of the HWPCB, which the executive writes and the PAL reads ([ADR-0011](../adr/0011-asts-on-the-kernel-stack.md)) |
| 0x09, 0x0A | `CSERVE`, `SWPPAL` | dropped: firmware services |
| 0x0B, 0x0C | `MFPR_FEN`, `MTPR_FEN` | kept, with floating point |
| 0x0D | `MTPR_IPIR` | kept, with more than one CPU |
| 0x0E, 0x0F | `MFPR_IPL`, `MTPR_IPL` | kept, implemented |
| 0x10, 0x11 | `MFPR_MCES`, `MTPR_MCES` | kept |
| 0x12 | `MFPR_PCBB` | kept, implemented |
| 0x13, 0x14 | `MFPR_PRBR`, `MTPR_PRBR` | kept |
| 0x15 | `MFPR_PTBR` | dropped: page tables are the PAL's |
| 0x16, 0x17 | `MFPR_SCBB`, `MTPR_SCBB` | kept, implemented |
| 0x18, 0x19 | `MTPR_SIRR`, `MFPR_SISR` | kept, implemented |
| 0x1A-0x1D, 0x24, 0x25 | `MFPR_TBCHK`, `MTPR_TBIA`, `MTPR_TBIAP`, `MTPR_TBIS`, `MTPR_TBISD`, `MTPR_TBISI` | changed: `WRPTE` |
| 0x1E-0x23 | `MFPR_ESP`, `MTPR_ESP`, `MFPR_SSP`, `MTPR_SSP`, `MFPR_USP`, `MTPR_USP` | kept |
| 0x29, 0x2A | `MFPR_VPTB`, `MTPR_VPTB` | dropped: no virtual page table |
| 0x2B, 0x2E | `MTPR_PERFMON`, `MTPR_DATFX` | dropped |
| 0x30-0x33 | `MFPR_VIRBND`, `MTPR_VIRBND`, `MFPR_SYSPTBR`, `MTPR_SYSPTBR` | dropped: page tables are the PAL's |
| 0x3E | `WTINT` | kept, implemented: the idle loop waits for an interrupt |
| 0x3F | `MFPR_WHAMI` | kept |

Unprivileged, 0x80-0xBF:

| Code | Calls | vaxpunk |
| --- | --- | --- |
| 0x80 | `BPT` | kept; today `BPT` is `brk`, an undefined-instruction fault |
| 0x81 | `BUGCHK` | kept |
| 0x82-0x85 | `CHME`, `CHMK`, `CHMS`, `CHMU` | changed, implemented: the PAL moves the call to the process's thread for that mode |
| 0x86 | `IMB` | kept |
| 0x87-0x8E, 0x93-0x9A, 0xA2-0xA9 | `INSQHIL` ... `REMQUEQ/D`, the resident forms | kept; `INSQUE` and `REMQUE` are inline code, since one thread runs at a time |
| 0x8F, 0x90 | `PROBER`, `PROBEW` | kept, implemented |
| 0x91, 0x92 | `RD_PS`, `REI` | kept, implemented |
| 0x9B, 0x9C | `SWASTEN`, `WR_PS_SW` | `SWASTEN` changed: `$SETAST` writes `ASTEN`; `WR_PS_SW` kept |
| 0x9D | `RSCC` | kept |
| 0x9E, 0x9F | `READ_UNQ`, `WRITE_UNQ` | dropped: user mode writes TPIDR_EL0 itself |
| 0xA0, 0xA1 | `AMOVRR`, `AMOVRM` | dropped: ARM64 has atomics |
| 0xAA | `GENTRAP` | kept |
| 0xAE | `CLRFEN` | kept, with floating point |

vaxpunk's own, 0x40-0x7F:

| Code | Call | Stands in for |
| --- | --- | --- |
| 0x40 | `MTPR_TXDB` | the VAX console transmit register |
| 0x41 | `WRPTE` | writing a PTE in memory, then `MTPR_TBIS` |
| 0x42 | `DELCTX` | the PAL's half of deleting a process |
| 0x43-0x45 | `MTPR_RXCS`, `MFPR_RXCS`, `MFPR_RXDB` | the VAX console receive registers |
| 0x46, 0x47 | `READLBLK`, `WRITELBLK` | Alpha's console `READ` and `WRITE` callbacks |
| 0x48 | `MTPR_DOORBELL` | a smart peripheral's doorbell, as an MSCP port's polling register |
