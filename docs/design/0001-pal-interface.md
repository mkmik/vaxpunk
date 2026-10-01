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
privileged instructions, and [ADR-0003](../adr/0003-one-cpu-many-threads.md),
which makes processes threads that take turns on one CPU, and
[ADR-0004](../adr/0004-interval-timer-is-a-pal-thread.md), which makes the
interval timer a thread of the PAL's.
[DESIGN-0002](0002-executive-processes.md) covers what the executive does
with it.

## The vocabulary rule

The PAL interface speaks only VMS: PCBs, PTEs, IPL, ASTs, access modes, the
system control block. It never speaks of capabilities, endpoints or
scheduling contexts. Whatever seL4 needs is translated inside the PAL. The
test for any new PAL call: could its signature have appeared in a DEC
internals manual? If it names a seL4 concept, it belongs inside the PAL, not
in its interface.

## What the PAL sets up

The root task (`roottask/src/main.c`) finds `EXEC.EXE` on the boot volume,
which the shim appends to the root task's image, and starts it. The
executive's address space, which every process shares:

| Address | What | Protection |
| --- | --- | --- |
| `0x10000` up | `EXEC.EXE`'s sections, at their link addresses | theirs |
| `0x1FFF0000` | the restart parameter block, one page | kernel write |
| `0x20000000` up | the boot volume | kernel read |
| `0x7FFEC000`-`0x7FFF0000` | the boot stack, 16 KB | kernel write |

Every page is a PFN of the executive's (*Memory*), and the PAL uses PFNs
from 0 up for these. Nothing else is mapped. The boot context starts with:

- **Registers.** PC is the transfer address. `sp` and x28 (VAX SP) point at
  the stack top. R11 points at the RPB, as the VAX's VMB passed it.
  Everything else is 0: AP is 0, as for a `CALLS` with no argument list,
  and so is the return address. A `RET` from the transfer routine
  therefore faults at 0.
- **Processor state.** Kernel mode, IPL 31, as a VAX starts. No SCB, no
  software interrupts pending. The clock is already ticking: its interrupt
  waits for IPL to drop below 24.
- **Process context.** Its HWPCB is the RPB's (`RPB$Q_HWPCB`).
- **Scheduling.** Its own scheduling context, with the root task's budget
  and period, and a priority one below the root task's.

### The restart parameter block

`$RPBDEF` in `vtools/lib/lib.mlb`, longwords:

| Offset | Field | Holds |
| --- | --- | --- |
| 0 | `RPB$L_BASE` | the RPB's address |
| 4 | `RPB$L_PFNCNT` | how many PFNs the executive has: 0 to `PFNCNT` - 1 |
| 8 | `RPB$L_FREEPFN` | the PFNs below this one hold what the PAL set up |
| 12 | `RPB$L_VOLUME` | the boot volume's address |
| 16 | `RPB$L_VOLSIZE` | its size in bytes |
| 20 | `RPB$L_BOOTTIME` | seconds since 1970 at boot, from QEMU virt's PL031 RTC, which the PAL maps after the UART and reads once |
| 64 | `RPB$Q_HWPCB` | the boot context's HWPCB, 128 bytes |

### The boot volume

The root task's Limine module `volume`, `sys.vol`, which `roottask/build.rs`
writes. Its first 512-byte block is a directory of 32-byte entries
(`$BVDDEF`): a `.ASCIC` name of up to 23 characters, the first block and the
size in bytes, ended by an empty name. Each file starts on a block.
ponytail: a stand-in for a Files-11 volume, until there is a disk driver.

## Calling the PAL

The executive calls the PAL with `svc #0`:

| Register | In | Out |
| --- | --- | --- |
| x7 | function code | unchanged |
| x0 | a0, the first argument | v0, the result, if the call has one |
| x1-x5 | a1-a5 | unchanged |
| x6, x8-x30, `sp`, NZCV | not seen by the PAL | unchanged |

Execution resumes after the `svc`, unless the PAL delivers an interrupt on
the way (*Interrupts and exceptions*), or the call is `REI`, `CHMK` or
`SWPCTX`.

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
    mode is a reserved instruction.
  - Today the executive is always in kernel mode, and the PAL stops it on a
    reserved instruction.

## Memory

The executive owns physical memory as VMS does, by page frame number, and
keeps the PFN database: which PFNs are free, which hold what. A PFN is a
4 KB page the PAL makes the first time a PTE names it, from the largest RAM
untyped. ponytail: 1024 PFNs, 4 MB, as many as the root CNode's 4096 slots
leave room for.

The executive maps a page by writing its PTE with `WRPTE`, which takes the
place of the VAX's writing a PTE in memory and then `MTPR #PR$_TBIS`
(ADR-0001: explicit calls instead of a page table the PAL walks). A PTE is
the VAX's longword (`$PTEDEF`), with one change:

| Bits | Field | Meaning |
| --- | --- | --- |
| 31 | `V` | valid: the page is mapped |
| 30:27 | `PROT` | the VAX protection code (`$PRTDEF`): who may read and write |
| 25 | `EXEC` | **new:** instructions may run from the page; ARM64 has the permission, the VAX had none |
| 20:0 | `PFN` | the page frame |

A valid PTE with protection `NA` maps nothing. `WRPTE` maps, unmaps or
changes protection at once, and returns the PTE the page had, so the
executive learns which PFN it freed. A PFN maps at one address at a time.
ponytail: a frame capability per extra mapping, for shared pages.

The PAL sees every PFN at an address of its own, the way Alpha PALcode read
memory physically. It reads and writes the executive's memory there to push
and pop interrupt frames and to read HWPCBs and the SCB.

## Process contexts

A process context is a thread in the executive's address space
([ADR-0003](../adr/0003-one-cpu-many-threads.md)). The executive names it by
its hardware PCB: a 128-byte, quadword-aligned block (`$HWPCBDEF`) whose
first quadword is the kernel stack pointer. Only the current context's
thread runs; `MFPR #PR$_PCBB` returns its HWPCB.

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

`DELCTX` (a0 = a HWPCB) deletes the context of a process that isn't
current, and stops its thread. The executive then frees the HWPCB and the
kernel stack. A HWPCB the PAL has no context for is fine.

## Interrupts and exceptions

The PAL delivers through the system control block, whose address the
executive sets with `MTPR #PR$_SCBB`. Its vectors are longwords at the
VAX's offsets (`$SCBDEF`): 0x40 for `CHMK`, 0x80 + 4n for software
interrupt level n, 0xC0 for the interval timer. To deliver, the PAL pushes
a frame on the current stack, 16-byte aligned and below both VAX SP and
ARM64's `sp`, and jumps to the vector:

| Offset | Holds |
| --- | --- |
| 0 | PC |
| 8 | PSL: IPL in bits 20:16, modes in 25:22 (0, kernel), NZVC in 3:0 |
| 16 | R7 |
| 24 | VAX SP before the frame |
| 32 | ARM64 `sp` |
| 40-80 | x13-x18, vmacro's scratch registers |
| 88 | x30 |

This is `$INTSTKDEF`. It holds what VAX code may have live that the code
it interrupts doesn't save: vmacro keeps operands and R0 and R7 in x13-x18
across a PAL call. A handler saves the VAX registers it uses, as on the
VAX, and returns with `REI`, which pops the frame, sets IPL and the
condition codes from the PSL and resumes at the PC. ponytail: `REI` to
kernel mode only, until there are other modes.

An interrupt can come between any two instructions, so a handler keeps
every register it uses, R0 included.

- **Software interrupts.** `MTPR #PR$_SIRR` requests level 1-15;
  `MFPR #PR$_SISR` shows which are pending. The PAL delivers the highest
  pending interrupt above IPL, at its level, when a call lowers IPL: `MTPR
  #PR$_IPL`, `REI`, a new context's start. A request above IPL is delivered
  on the `MTPR` that makes it.
- **The interval timer.** Every 10 ms the PAL requests an interrupt at
  IPL 24, the VAX's interval timer's, through vector 0xC0. If IPL is below
  24, the PAL delivers it at once: it stops the current context wherever it
  is and resumes it at the handler, or, if it waits in `WTINT`, returns from
  that. Otherwise it stays pending, like a software interrupt, until IPL
  drops below 24. A tick while one is pending is lost, as on the VAX. The
  clock runs from boot; it has no `ICCS` to enable it. ponytail: the tick
  is a thread of the PAL's that seL4 wakes each period (ADR-0004); a timer
  driver replaces it on hardware that has a free timer.
- **`CHMK`.** The code in R0. The PAL pushes the frame and jumps to the SCB's
  `CHMK` vector at the same IPL. The handler leaves the service's status in
  R0, which `REI` doesn't change.
- **`WTINT`.** Returns once an interrupt IPL lets through has been
  delivered: at once if one is pending, or at the next tick.

### Faults and HALT

The executive's faults go to the PAL too. For now each of them stops the
executive, as `HALT` does, naming the HWPCB of the process that took it:

| Event | The PAL prints |
| --- | --- |
| Page fault | `%PAL-F-ACCVIO` with the address and PC |
| Undefined instruction, `brk` (`BPT`) | `%PAL-F-OPCDEC` with the PC |
| Reserved PAL call, bad `WRPTE` | `%PAL-F-OPCDEC` with the code, PC, R0 and R1 |
| Delivery with no vector or stack | `%PAL-F-NOVEC` |
| `REI` without a frame | `%PAL-F-REI` |
| `HALT` | `%PAL-I-HALT` with the PC and R0 |

The root task then prints `root task done` and suspends itself. Later, the
PAL reflects faults to the executive as access violations and reserved
instructions through the SCB, as on Alpha.

## How MACRO-32 calls it

vmacro compiles privileged instructions into PAL calls, as AMACRO compiled
them into `CALL_PAL` (`vtools/docs/macro32.md`). The executive writes VAX
source:

```
        .LIBRARY "lib.mlb"
        $PRDEF
        MTPR    #8, #PR$_IPL
```

and gets:

```
        movz    w14, #8         ; the operand
        mov     x15, x0         ; R0 and R7 aside
        mov     x16, x7
        mov     w0, w14         ; a0: the new IPL
        mov     x7, #15         ; MTPR_IPL
        svc     #0
        mov     x17, x0         ; v0: the old IPL, unused by MTPR
        mov     x7, x16
        mov     x0, x15
```

- **Registers.** R0 and R7 are x0 and x7, so vmacro keeps them in scratch
  registers around the call. Every other register, and the VAX's view of
  R0 and R7, comes back unchanged.
- **Constants.** The processor register must be a constant, as in AMACRO.
  `$PRDEF` in `vtools/lib/lib.mlb` defines the VAX's `PR$_` names.

| VAX instruction | PAL call |
| --- | --- |
| `MTPR src, #PR$_IPL` | `MTPR_IPL`, a0 = src |
| `MFPR #PR$_IPL, dst` | `MFPR_IPL`, dst = v0 |
| `MFPR #PR$_PCBB, dst` | `MFPR_PCBB` |
| `MTPR src, #PR$_SCBB`, `MFPR #PR$_SCBB, dst` | `MTPR_SCBB`, `MFPR_SCBB` |
| `MTPR src, #PR$_SIRR` | `MTPR_SIRR` |
| `MFPR #PR$_SISR, dst` | `MFPR_SISR` |
| `MTPR src, #PR$_TXDB` | `MTPR_TXDB` |
| `CHMK #code` | `CHMK`, R0 = code; R0 isn't kept |
| `REI` | `REI` |
| `HALT` | `HALT` |
| `CALL_PAL #code` | any call: a0-a5 in R0-R5, v0 in R0, as on Alpha |

`CALL_PAL` reaches the calls the VAX has no instruction for: `SWPCTX`,
`WTINT`, `WRPTE`, `DELCTX`. `$PALDEF` in `lib.mlb` names their codes.

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
| 0x83 | `CHMK` | Alpha | a0 = code | | delivers through the SCB |
| 0x92 | `REI` | Alpha | | | pops a frame: PC, PSL, registers |

`MTPR_TXDB` stands in for the VAX console transmit register: VMS's
`CON$PUTCHAR` writes each character with `MTPR R0, #PR$_TXDB`. Alpha had no
such register; its console output went through firmware callbacks.
The PAL writes it on the PL011 UART, which it maps from its device untyped
and drives itself, so the console needs no debug seL4. ponytail: QEMU
virt's UART address, polled; from the DTB, with interrupts, later.

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
| 0x07, 0x08, 0x26, 0x27 | `MTPR_ASTEN`, `MTPR_ASTSR`, `MFPR_ASTEN`, `MFPR_ASTSR` | kept |
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
| 0x82-0x85 | `CHME`, `CHMK`, `CHMS`, `CHMU` | changed: the PAL moves the call to the task for that mode; `CHMK` implemented, from kernel mode |
| 0x86 | `IMB` | kept |
| 0x87-0x8E, 0x93-0x9A, 0xA2-0xA9 | `INSQHIL` ... `REMQUEQ/D`, the resident forms | kept; `INSQUE` and `REMQUE` are inline code, since one thread runs at a time |
| 0x8F, 0x90 | `PROBER`, `PROBEW` | kept |
| 0x91, 0x92 | `RD_PS`, `REI` | kept; `REI` implemented |
| 0x9B, 0x9C | `SWASTEN`, `WR_PS_SW` | kept |
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
