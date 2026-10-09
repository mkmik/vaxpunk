# C examples

A BLISS-64 program calling a C library: what a port of a C library to
vaxpunk looks like, from a stock gcc to an object library to a VMS
program, and what has to be adapted between C's calling conventions and
VMS's. [docs/velf.md](../../docs/velf.md) describes `velf`, which turns
gcc's ELF objects into object modules.

```sh
cd crosstools/vtools/examples/c
just cdemo              # builds the tools and SYSLIB.OLB, then compiles, links and runs cdemo
just --dry-run cdemo    # shows the commands without running them
```

The cross gcc is `aarch64-elf-gcc` (`CROSS_COMPILE` changes the prefix).
`cargo test -p vrun --test programs c` builds and runs it too, and checks
its output against
[tests/examples/c/cdemo.stdout](../../tests/examples/c/cdemo.stdout).

## cdemo

| File | What |
| --- | --- |
| [cdemo/lib/cdemo.c](cdemo/lib/cdemo.c) | CDEMO, the C library: plain C with C's conventions |
| [cdemo/lib/cdemo_vms.b64](cdemo/lib/cdemo_vms.b64) | CDEMO_VMS, its VMS face in BLISS-64: `CDEMO$SUM`, `CDEMO$NAME` and `CDEMO$WORDS` take descriptors and return condition values |
| [cdemo/main.b64](cdemo/main.b64) | The program, which calls the VMS face, and C once directly |

```
cdemo.c ──gcc──> cdemo.o ──velf──> cdemo.obj ─┐
cdemo_vms.b64 ──vbliss──> cdemo_vms.obj ──────┴─vlib──> LIB.OLB ─┐
main.b64 ──vbliss──> main.obj ────────────────────────────vlink──┴─> CDEMO.EXE
```

## What crosses and what doesn't

The vaxpunk calling standard is AAPCS64's
([ADR-0023](../../../../docs/adr/0023-calling-standard.md)), so BLISS-64
calls C, and C calls BLISS-64, with no jacket in between. What each side
must still do:

| | What happens | What to do | Where cdemo shows it |
| --- | --- | --- | --- |
| Arguments | Both put the first eight in x0-x7 and the rest on the stack, 8 bytes each | Nothing | `CDEMO$SUM`'s twelve arguments, eleven to `cdemo_sum` |
| Narrow arguments | C extends a `char`, `short` or `int` itself, from the low bits of the register or slot | Nothing | `cdemo_sum`'s signed and unsigned bytes and words |
| The count in x9 | BLISS-64 sets it, C ignores it | Nothing, BLISS-64 to C | every call |
| `int` results | C leaves the upper half of x0 zero, not the sign: `-1` reads as 4294967295 | Take `.R<0, 32, 1>` | `cdemo_name 12 as a quadword` in the output |
| Results through pointers | C writes as many bytes as its type has: an `int` is a longword | Declare the variable as wide, `LONG SIGNED`, or extract | `MINIMUM`, `COUNT` |
| Strings | C takes an address and a length | Take them out of the descriptor | `CDEMO$NAME`, `CDEMO$WORDS` |
| Errors | C returns a negative number | Turn it into a condition value | `SS$_BADPARAM` from `CDEMO$NAME 12` |
| Calls from C | No count in x9, and an `int` argument's upper half is anything | The routine C calls never uses `ACTUALCOUNT`, and takes `<0, 32, 1>` of its `int`s | `EMIT` |
| VMS routines from C | A routine that reads its count, as every system service does, finds x9 is whatever C left there | C calls only BLISS-64 routines like `EMIT`, which call VMS's | `EMIT` calls the action routine |
| Conditions | C's frames have no frame descriptor: at 16(FP) and 24(FP), where the handler search looks, gcc keeps saved registers | Every routine C calls catches every condition and unwinds to C with an error | `EMIT`'s handler, `STOP`, and PANIC in the output |
| Data | C's globals are ordinary symbols, and C's read-only tables of addresses go in `$READONLY_ADDR$`, which gets fixups when the image moves | Nothing | `CDEMO_CALLS`, `names[]` |
| x18 | The VAX stack pointer | `-ffixed-x18` | velf's flags |

Without `STOP`, a condition the action routine signals is searched for
through `cdemo_words`'s frame, and the image ends with an access
violation in `EXE$SIGNAL`, which took a saved register for a frame
descriptor. `$UNWIND` back into C is safe: it restores the registers C
keeps from the frame descriptors of the frames it removes. After PANIC,
`cdemo_words` writes where it stopped from two of them, x19 and x26,
and `stopped at 7` is right.

## For a real library

Mbed TLS 4.1.1, compiled with the same flags, converts whole: 110
modules, whose relocations are all ones velf takes, one named with
`/OBJECT` since its file's name is longer than 31 characters. Beyond
itself it needs only `memcpy`, `memmove`, `memset`, `memcmp`, `strlen`,
`strcmp`, `strncmp`, `strchr` and `strstr`, libgcc's `__udivti3`, which
velf converts from `libgcc.a`, and the platform's hooks. Its I/O
callbacks and those hooks (entropy from `$GET_ENTROPY`, time from
`$GETTIM`, memory) are BLISS-64 routines C calls, as `EMIT` is, and its
error codes become condition values, as `cdemo_name`'s -1 does.
