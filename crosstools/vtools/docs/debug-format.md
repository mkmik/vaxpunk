# Debug symbol table format

The debug symbol table (DST) tells DEBUG and traceback what the machine code
was in the source: the module, its routines and blocks, which address each
line starts at, the source files, and the symbols and their types. vaxpunk's
is OpenVMS Alpha's, with no change: nothing in it depends on the instruction
set. A compiler writes DST records in an object module's EDBG and ETBT
records (`docs/object-format.md`); the linker gathers them into the image's
DST and adds a debug module table (DMT) that says where each module's
records are (`docs/image-format.md`).

The code is `vms-obj/src/dst.rs` for the records and `vms-obj/src/exe.rs`
for the image's tables (`Image::debug`).

Status: the format only. `vmacro`, `vbliss` and `vlink` write no DST yet,
and there is no traceback or DEBUG on vaxpunk to read one
(`t8y:/utils/debugger`).

## Sources

- GNU binutils: the record layouts in `include/vms/dst.h`, `dmt.h` and
  `eihs.h`, and how `bfd/vms-alpha.c` reads them and builds an image's DST
  and DMT. GPL-3: facts only, never code.
- VAX/VMS V4.3's DEBUG and TRACE sources, `DEBUG/SRC/DSTRECRDS.REQ` and
  `TRACE/SRC/TBKDPC.B32`: the meaning of every record and of each
  PC-correlation command. DEC's: read for facts, never copied.
- OpenVMS Alpha V8.4 itself (the AXPbox oracle, `t8y:/host/axpbox-oracle`):
  objects from `MACRO/MIGRATION/DEBUG` and `BLISS/A64`, and images from
  `LINK/DEBUG`, `LINK` and `LINK/NOTRACEBACK`. Where the others don't say,
  or disagree, these decide. The tests in `dst.rs` and `exe.rs` hold
  pieces of them.

## Records

Integers are little-endian. A DST is records one after another. Each starts
with a 2-byte length and a 2-byte type:

| Offset | Size | Field |
| --- | --- | --- |
| 0 | 2 | `DST$W_LENGTH`: the record's size **minus one** |
| 2 | 2 | `DST$W_TYPE` |
| 4 | | the fields of that type |

The length is one short because VAX's DST had a 1-byte length that didn't
count itself, and Alpha widened the field but kept the count: a `MODEND`
record, 4 bytes, has length 3. A length of 0 ends an image's DST.

`vms-obj` decodes the records that say where code is; the others (symbols,
types, enumerations, ...) it keeps as their bytes.

| Type | Name | Fields after the type |
| --- | --- | --- |
| 188 | `MODBEG` | flags (1), unused (1), language (4), DST version major (2) and minor (2), module name (ASCIC), compiler ident (ASCIC) |
| 189 | `MODEND` | none |
| 190 | `RTNBEG` | flags (1), address of the first instruction (4), address of the procedure descriptor (4), routine name (ASCIC) |
| 191 | `RTNEND` | unused (1), the routine's size in bytes (4) |
| 176 | `BLKBEG` | unused (1), address (4), block name (ASCIC) |
| 177 | `BLKEND` | unused (1), the block's size in bytes (4) |
| 162 | `PROLOG` | the address where the routine's prologue ends, where a breakpoint on it goes (4) |
| 185 | `LINE_NUM` | PC-correlation commands, up to the end |
| 155 | `SOURCE` | source correlation commands, up to the end |

A module's records start with `MODBEG` and end with `MODEND`; a routine's
and a block's nest between their `BEG` and `END`. Languages: 0 MACRO, 2
BLISS, 7 C; AMACRO, MACRO-32 for Alpha, writes 16. The V8.4 compilers write
version 1.18.

Addresses are longwords, as on Alpha. An image whose code lies outside the
lowest or the highest 2 GB (sign-extended) can't be described; the store
commands that put them in fail the link instead (`STO_LW`'s check).

## Line numbers

A `LINE_NUM` record holds PC-correlation commands: a command byte and at
most one argument. A command byte from -128 to 0 is a Delta-PC with no
argument: minus the byte is how far the next line starts from the current
PC. All the `LINE_NUM` records of a module are one program, run with this
state: the line number (0 at first), the line increment (1), the statement
number (1) and statement mode (off), the PC (the address of the module's
lowest routine), and whether a line is open (no).

| Code | Name | Argument | What it does |
| --- | --- | --- | --- |
| -128..0 | Delta-PC | | add the increment to the line number (in statement mode, 1 to the statement number), add minus the code to the PC: that line starts there, and is open |
| 1 | `DELTA_PC_W` | word | the same, by the argument |
| 17 | `DELTA_PC_L` | longword | the same, by the argument |
| 2, 3, 18 | `INCR_LINUM`, `_W`, `_L` | byte, word, longword | add to the line number |
| 4, 5 | `SET_LINUM_INCR`, `_W` | byte, word | set the increment |
| 6 | `RESET_LINUM_INCR` | | the increment is 1 again |
| 7 | `BEG_STMT_MODE` | | statement mode on; only in an open line |
| 8 | `END_STMT_MODE` | | statement mode off |
| 13 | `SET_STMTNUM` | word | set the statement number |
| 19, 9, 20 | `SET_LINUM_B`, `SET_LINUM`, `SET_LINUM_L` | byte, word, longword | set the line number |
| 10, 11, 12 | `SET_PC`, `_W`, `_L` | byte, word, longword | the PC is the module's lowest routine's address plus this |
| 16 | `SET_ABS_PC` | longword | set the PC |
| 14, 15, 21 | `TERM`, `_W`, `_L` | byte, word, longword | the open line has this many bytes; no line is open |

A line ends where the next Delta-PC starts one, or at a `TERM`. The commands
that set the line number or the PC come only while no line is open. Those
that change the line number or the increment set the statement number back
to 1 in statement mode. `dst::lines` runs the commands and returns each
line's number, statement, address and size, as TRACE reads them.

This is what AMACRO wrote for a routine at 0x30040 whose `.ENTRY` is on line
10 and whose four instructions are on lines 11 to 14, the last ending at
0x30094:

```
SET_ABS_PC 0x30040                      (its own LINE_NUM record)
SET_LINUM_B 9
Delta-PC 0      line 10 at 0x30040
Delta-PC 28     line 11 at 0x3005C
Delta-PC 12     line 12 at 0x30068
Delta-PC 12     line 13 at 0x30074
Delta-PC 4      line 14 at 0x30078
TERM 28         the routine ends at 0x30094
```

## Source files

A `SOURCE` record holds source correlation commands, a command byte and its
arguments, which say which records of which files the module's line numbers
stand for:

| Code | Name | Arguments |
| --- | --- | --- |
| 1 | `SRC_DECLFILE` | a length byte (the bytes after it), flags (1), file id (2), the file's creation time (8), end of file block (4), first free byte (2) and organization and record format (1), as RMS gives them, the file name (ASCIC), and the module's name if the file is a text library's (ASCIC) |
| 2 | `SRC_SETFILE` | the file id the next lines come from (2) |
| 3, 4 | `SRC_SETREC_L`, `_W` | the record of the file the next line is (4 or 2) |
| 5, 6 | `SRC_SETLNUM_L`, `_W` | the next line's number (4 or 2) |
| 7 | `SRC_INCRLNUM_B` | add to it (1) |
| 10, 11 | `SRC_DEFLINES_W`, `_B` | that many lines are the next records (2 or 1) |
| 16 | `SRC_FORMFEED` | form feeds count as records |

For the module above AMACRO declared `SYS$SYSDEVICE:[ORACLE]DBGM.MAR;1` as
file 1, then `SRC_SETLNUM_L 1`, `SRC_SETFILE 1`, `SRC_SETREC_L 1`,
`SRC_DEFLINES_W 15`: lines 1 to 15 are its records 1 to 15.

## In an object module

EDBG (12) and ETBT (13) records hold TIR commands, the same as ETIR
records', which build the module's DST instead of its psects' contents: the
linker runs each module's EDBG and ETBT records, in order, on one location
counter that starts at the module's place in the image's DST. `STO_IMM`
stores the records' bytes, and an address is a `STA_PQ` and a `STO_LW`.

ETBT records carry what traceback needs: the module, its routines and its
line numbers. EDBG records carry the rest for DEBUG: source files, symbols
and types. `LINK/DEBUG` puts both in the image; `LINK`, which is
`/TRACEBACK`, only the ETBT records; `/NOTRACEBACK` neither.

`CTL_DFLOC`, `CTL_STLOC` and `CTL_STKDL` let a compiler fill in a record it
wrote earlier. `CTL_DFLOC` pops an index and keeps the location counter
under it, `CTL_STLOC` pops one and sets the location counter to what is kept
there, and `CTL_STKDL` pops one and pushes what is kept there. AMACRO starts a module with
`STA_LW 0`, `CTL_DFLOC`; at its end it stores `STA_LW 1`, `CTL_DFLOC`, goes
back with `STA_LW 0`, `CTL_STLOC` and `CTL_AUGRB` to the labels whose
addresses it knows only then, and returns with `STA_LW 1`, `CTL_STLOC` to
write `MODEND`. BLISSA64 uses `CTL_STKDL` to store the offset of one record
in another.

## In an image

The image symbol table header (`EIHS$`, `docs/image-format.md`) says where
the DST and the DMT are. The Alpha linker writes the DST from a block
boundary after the sections, with `DSTSIZE` its size in whole blocks, the
records ended by a zero length; then the DMT from the next block, with
`DMTBYTES` its exact size; then the GST. `EIHD$L_LNKFLAGS` has `LNKDEBUG`
(1) if the image was linked /DEBUG, so the debugger starts with it, and
`DBGDMT` (0x20) if it has a DMT.

The DMT has an entry per module with DST records:

| Offset | Size | Field |
| --- | --- | --- |
| 0 | 4 | `DMT$L_MODBEG`: where the module's records start in the DST |
| 4 | 4 | `DMT$L_SIZE`: their size in bytes |
| 8 | 2 | `DMT$W_PSECT_COUNT`: how many psect entries follow |
| 10 | 2 | 0 |
| 12 | 8 each | a psect entry: the address (4) and length (4) of the module's contribution to each of its psects |

So the debugger finds the module that holds an address from the DMT, and
reads only that module's records.
