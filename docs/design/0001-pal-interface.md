# DESIGN-0001 — PAL interface

Oct 1, 2026 · @Marko Mikulicic

The interface between the vaxpunk executive and the PAL below it:
- how the executive calls the PAL;
- what each call does;
- what the PAL sets up before the executive runs.

It builds on [ADR-0001](../adr/0001-pal-interface-vms-vocabulary.md), which
sets the vocabulary, and [ADR-0002](../adr/0002-root-task-is-the-pal.md),
which makes the root task the PAL and the executive a task calling it with
privileged instructions.

## The vocabulary rule

The PAL interface speaks only VMS: PCBs, PTEs, IPL, ASTs, access modes, the
system control block. It never speaks of capabilities, endpoints or
scheduling contexts. Whatever seL4 needs is translated inside the PAL. The
test for any new PAL call: could its signature have appeared in a DEC
internals manual? If it names a seL4 concept, it belongs inside the PAL, not
in its interface.

## The executive task

The root task (`roottask/src/main.c`) starts the executive from `exec.exe`,
which the shim appends to the root task's image. The executive gets:

- **Address space.**
  - The image's sections at their link addresses, with their protection.
  - 16 KB of stack below 0x7FFF0000, under 2 GB, where MACRO-32's longword
    addresses reach.
  - Nothing else is mapped.
- **Registers.**
  - PC is the transfer address.
  - `sp` and x28 (VAX SP) point at the stack top.
  - Everything else is 0: AP is 0, as for a `CALLS` with no argument list,
    and so is the return address. A `RET` from the transfer routine
    therefore faults at 0.
- **Processor state.** Kernel mode, IPL 31, as a VAX starts.
- **Scheduling.** Its own scheduling context, with the root task's budget
  and period, and a priority one below the root task's.

## Calling the PAL

The executive calls the PAL with `svc #0`:

| Register | In | Out |
| --- | --- | --- |
| x7 | function code | unchanged |
| x0 | a0, the first argument | v0, the result, if the call has one |
| x1-x5 | a1-a5 | unchanged |
| x6, x8-x30, `sp`, NZCV | not seen by the PAL | unchanged |

Execution resumes after the `svc`.

- **Registers.** Alpha passed a0-a5 in R16-R21 and v0 in R0. vaxpunk uses
  x0-x5 because seL4 passes the PAL only x0-x7. A call that leaves x0 alone
  returns it unchanged.
- **How it travels.** seL4 reads its syscall number from x7 and ignores the
  `svc` immediate. Every PAL code is non-negative, and seL4's own numbers
  are negative, so a PAL call reaches the PAL as an unknown syscall fault.
  - The PAL gets x0-x7, the PC of the `svc`, SP, LR and SPSR.
  - It replies with x0-x7 and the PC to resume at: the `svc`'s address
    plus 4. seL4 would otherwise run the `svc` again.
- **Privileged calls.**
  - Codes 0x00-0x7F are privileged: kernel mode only.
  - Codes 0x80-0xBF are for any mode.
  - Any other code, an unimplemented one, or a privileged call from another
    mode is a reserved instruction.
  - Today the executive is always in kernel mode, and the PAL stops it on a
    reserved instruction.

### Faults and HALT

The executive's faults go to the PAL too. For now each of them stops the
executive, as `HALT` does:

| Event | The PAL prints |
| --- | --- |
| Page fault | `%PAL-F-ACCVIO` with the address and PC |
| Undefined instruction, `brk` (`BPT`) | `%PAL-F-OPCDEC` with the PC |
| Reserved PAL call | `%PAL-F-OPCDEC` with the code and PC |
| `HALT` | `%PAL-I-HALT` with the PC and R0 |

The root task then prints `root task done` and suspends itself. Later, the
PAL resolves a page fault from the executive's PTEs, or reflects it to the
executive as an access violation through its system control block, as on
Alpha.

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
| `MTPR src, #PR$_TXDB` | `MTPR_TXDB`, a0 = src |
| `HALT` | `HALT` |

## Function codes

The codes are Alpha OpenVMS PALcode's (Alpha Architecture Reference Manual,
OpenVMS software part), checked against the VMS PALcode in SIMH's Alpha
simulator. vaxpunk's own calls start at 0x40, a range Alpha OpenVMS leaves
unused.

### Implemented

| Code | Call | Origin | In | Out | What the PAL does |
| --- | --- | --- | --- | --- | --- |
| 0x00 | `HALT` | Alpha | | | stops the executive; prints its PC and R0 |
| 0x0E | `MFPR_IPL` | Alpha | | v0 = IPL | |
| 0x0F | `MTPR_IPL` | Alpha | a0 = new IPL | v0 = old IPL | keeps the IPL, 0-31; nothing is masked yet |
| 0x40 | `MTPR_TXDB` | vaxpunk | a0 = character | | writes a0's low byte on the console |

`MTPR_TXDB` stands in for the VAX console transmit register: VMS's
`CON$PUTCHAR` writes each character with `MTPR R0, #PR$_TXDB`. Alpha had no
such register; its console output went through firmware callbacks.
ponytail: the PAL writes with `seL4_DebugPutChar`, which only a debug seL4
has; a UART driver in the PAL replaces it.

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
| 0x05 | `SWPCTX` | changed: create, switch and delete process contexts |
| 0x06 | `MFPR_ASN` | dropped: address space numbers are seL4's |
| 0x07, 0x08, 0x26, 0x27 | `MTPR_ASTEN`, `MTPR_ASTSR`, `MFPR_ASTEN`, `MFPR_ASTSR` | kept |
| 0x09, 0x0A | `CSERVE`, `SWPPAL` | dropped: firmware services |
| 0x0B, 0x0C | `MFPR_FEN`, `MTPR_FEN` | kept, with floating point |
| 0x0D | `MTPR_IPIR` | kept, with more than one CPU |
| 0x0E, 0x0F | `MFPR_IPL`, `MTPR_IPL` | kept, implemented |
| 0x10, 0x11 | `MFPR_MCES`, `MTPR_MCES` | kept |
| 0x12 | `MFPR_PCBB` | kept |
| 0x13, 0x14 | `MFPR_PRBR`, `MTPR_PRBR` | kept |
| 0x15 | `MFPR_PTBR` | dropped: page tables are the PAL's |
| 0x16, 0x17 | `MFPR_SCBB`, `MTPR_SCBB` | kept: where the PAL delivers interrupts and exceptions |
| 0x18, 0x19 | `MTPR_SIRR`, `MFPR_SISR` | kept |
| 0x1A-0x1D, 0x24, 0x25 | `MFPR_TBCHK`, `MTPR_TBIA`, `MTPR_TBIAP`, `MTPR_TBIS`, `MTPR_TBISD`, `MTPR_TBISI` | changed: explicit map, unmap and protect calls |
| 0x1E-0x23 | `MFPR_ESP`, `MTPR_ESP`, `MFPR_SSP`, `MTPR_SSP`, `MFPR_USP`, `MTPR_USP` | kept |
| 0x29, 0x2A | `MFPR_VPTB`, `MTPR_VPTB` | dropped: no virtual page table |
| 0x2B, 0x2E | `MTPR_PERFMON`, `MTPR_DATFX` | dropped |
| 0x30-0x33 | `MFPR_VIRBND`, `MTPR_VIRBND`, `MFPR_SYSPTBR`, `MTPR_SYSPTBR` | dropped: page tables are the PAL's |
| 0x3E | `WTINT` | kept: the idle loop waits for an interrupt |
| 0x3F | `MFPR_WHAMI` | kept |

Unprivileged, 0x80-0xBF:

| Code | Calls | vaxpunk |
| --- | --- | --- |
| 0x80 | `BPT` | kept; today `BPT` is `brk`, an undefined-instruction fault |
| 0x81 | `BUGCHK` | kept |
| 0x82-0x85 | `CHME`, `CHMK`, `CHMS`, `CHMU` | changed: the PAL moves the call to the task for that mode |
| 0x86 | `IMB` | kept |
| 0x87-0x8E, 0x93-0x9A, 0xA2-0xA9 | `INSQHIL` ... `REMQUEQ/D`, the resident forms | kept |
| 0x8F, 0x90 | `PROBER`, `PROBEW` | kept |
| 0x91, 0x92 | `RD_PS`, `REI` | kept |
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
