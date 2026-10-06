# What an image sees under vrun

vrun runs an image in user mode (EL0) on a QEMU `virt` machine with no OS. A
small boot stub at EL1 sets up memory, starts the image, and serves a few
monitor calls. This is the contract between the two. All of it is provisional:
the calling standard, once it exists, replaces the entry convention.

## Machine

AArch64, little-endian, EL0, MMU and caches on, interrupts masked. QEMU's
`-cpu max` under TCG, `-cpu host` under HVF. FP and SIMD are enabled. Timers,
counters and interrupts are not available.

EL0 may use `DC ZVA`, cache maintenance instructions and `CTR_EL0`. `WFI`,
`WFE` and any access to an EL1 register trap, and so does every other
privileged instruction.

## Address space

The image's sections are at their link addresses, or moved (see *Moving the
image*), with the protection their section flags give. Nothing else in the
lower half is mapped except:

| Virtual address | What | Access |
| --- | --- | --- |
| `7FF00000` | runner info block | read |
| `7FF01000` | return page | execute |
| `7FF10000`-`7FFEFFFF` | stack, 896 KB | read, write |

The runner's reserved range is `7FF00000`-`7FFFFFFF`, at the top of VMS P1
space. The stub also reserves `40200000`-`4021FFFF` and the UART page at
`09000000`; the image can't access them. An image that overlaps any of these
fails to load. Page 0 is unmapped, and so is everything below the stack down to
`7FF02000`, which acts as a guard.

## Entry

| Register | Value |
| --- | --- |
| `pc` | the image's first transfer address |
| `x0` | address of the runner info block |
| `x9` | 1, the argument count |
| `x18` | `7FFF0000`, VAX SP, as `sp` |
| `x30` | address of the return page |
| `sp` | `7FFF0000`, 16-byte aligned |
| everything else | 0, including FP/SIMD registers, `FPCR` and `FPSR` |

The image is called as a routine with one argument, the info block, by the
calling standard (DESIGN-0004): `4(AP)` in a MACRO-32 main routine.

## Runner info block

| Offset | Size | Field |
| --- | --- | --- |
| 0 | 8 | size of the block, 32 |
| 8 | 4 | runner version, 1 |
| 12 | 4 | flags, 0 |
| 16 | 8 | argument string: a VMS static text descriptor (`DSC$W_LENGTH`, `DSC$B_DTYPE` = 14, `DSC$B_CLASS` = 1, 32-bit `DSC$A_POINTER`) |
| 24 | 8 | how far vrun moved the image from its link address, signed; 0 if it didn't |

The argument string is the rest of vrun's command line after the image name,
joined with single spaces. It follows the block, in the same page, at most 4064
bytes.

## Moving the image

`vrun --base ADDRESS` loads the image with its lowest section at `ADDRESS`
instead, a multiple of 64 KB, moving every section by the same amount. Only an
image with a fixup section can move (`vlink /RELOCATABLE`, see
`docs/image-format.md`); vrun applies the fixups before it builds the page
tables. It refuses a base that would put a section on the ranges above, or a
longword address beyond 2 GB.

Fault messages show the moved addresses. The map's symbols move with the
image, so the places they name are the same as without `--base`.

## Monitor calls

`SVC #n` from the image traps to the stub. Registers are preserved except as
listed.

| Call | Arguments | Effect |
| --- | --- | --- |
| `SVC #1` exit | `x0` status | ends the run |
| `SVC #2` put | `x0` address, `x1` length | writes the bytes to the console |
| `SVC #3` dump | | prints `x0`-`x30`, `sp` and the `pc` of the `SVC` |

The stub reads `put` data with unprivileged loads. A bad address is the
image's access violation, reported at the `SVC`. Any other `SVC` number is a
fault. `BRK` is left to debuggers.

The return page holds a single `SVC #1`, so returning from the entry point
exits with `x0` as the status.

## Exit status

The status is a VMS condition value: low bit set means success. vrun exits
with 0 for success, otherwise with the status's low byte (1 if that is 0).
`--verbose` prints the full 32-bit status.

## Faults

The stub reports faults to vrun, which prints a VMS-style message on stderr
and exits as if the image had returned this status:

| Exception | Message | Status |
| --- | --- | --- |
| data or instruction abort | `%VRUN-F-ACCVIO` with the address and PC | `SS$_ACCVIO`, %X0000000C |
| undefined, privileged or trapped instruction, unknown `SVC` | `%VRUN-F-OPCDEC` with the PC | `SS$_OPCDEC`, %X0000043C |
| anything else | `%VRUN-F-EXCEPT` with the syndrome | `SS$_ABORT`, %X0000002C |

Lines follow that say where: the PC and, for an access violation, the
address, each as image section and offset. With `--map` and the image's link
map (`vlink /MAP`), they also give the nearest global symbol before it in the
same section:

```
%VRUN-F-ACCVIO, access violation, virtual address=0000000000000008, PC=0000000000010004
-VRUN-I-PC, PC is START+%X4 (image section 1 + %X4)
```

An address in the unmapped pages below the stack means the stack overflowed,
and `-VRUN-I-STACKOVF` says so.

Then a line for each frame of the calling standard's
([DESIGN-0004](../../docs/design/0004-calling-standard.md)) on the image's
FP chain, from the innermost out, named by its descriptor, with where it
returns to. The walk stops at an FP of 0, at a frame whose descriptor
doesn't name a routine, and at memory the image can't read:

```
-VRUN-I-FRAME, INNER's frame, which returns to MIDDLE+%X1C (image section 1 + %X80)
-VRUN-I-FRAME, START's frame, which returns to PC=000000007FF01000
```

A hung image is stopped after `--timeout` seconds (default 30).

## Debugging

`vrun --gdb` starts QEMU halted, with a GDB server on port 1234, and prints
commands that attach a debugger and stop at the image's transfer address. With
LLDB, Xcode's included:

```
lldb -o 'gdb-remote 1234' -o 'breakpoint set -a 0x10000' -o continue
```

The debugger starts at the stub's first instruction, at 40200000. After the
breakpoint, `si` steps through the image at EL0; addresses are the image's
virtual addresses. The debugger has no symbols; the map does. There is no
timeout, and quitting the debugger ends the run.

## Console protocol

Everything the image writes goes to vrun's standard output unchanged. The stub
ends the run with one report, which vrun removes: `!vrun exit STATUS`,
`!vrun fault ESR PC FAR`, then a return address and a routine name for each
frame the stub found, or `!vrun stubfault ESR PC FAR` for a bug in the stub
itself, with 16-digit hex values. The report may follow the image's output on
the same line. An image must not write `!vrun ` itself.
