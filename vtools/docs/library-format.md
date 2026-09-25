# ARM64 object library format

A vaxpunk object library (OLB) is laid out like an OpenVMS Alpha object
library, library format 3.0: 512-byte blocks holding a header, two indexes and
the modules. Nothing in it is specific to the instruction set, so nothing
changed: the modules are object modules (`docs/object-format.md`), and each
says its own architecture. The code is `vms-obj/src/olb.rs`. `vlib` writes
libraries, `vlink` searches them (`docs/linker.md`), and `vdump` shows them.

Status: work order step 9. Only object libraries, and only what the librarian
and the linker need: no update history, no free block lists, no compression.

## Sources

The Linker Utility Manual (see `docs/object-format.md`) says what a library
holds, a module list and a name table (section 1.2.3), and which symbols go in
the name table (section 2.5), but not the layout. The layouts here come from
GNU binutils, which reads and writes Alpha libraries (`include/vms/lbr.h`,
`bfd/vms-lib.c`). Binutils is GPL-3, so this repository takes facts from it,
never code.

## Layout

```
block 1       library header (LHD), then an index descriptor (IDD) per index
blocks 2..    the module name index, then the symbol index
then          each module's data, starting on a new block
```

Blocks are numbered from 1. A place in the file is an RFA: a 4-byte block
number and a 2-byte offset in the block. Integers are little-endian.

## Library header (`LHD$`)

| Offset | Field | vaxpunk value |
| --- | --- | --- |
| 0 | `TYPE` | 7, `LBR$C_TYP_EOBJ`: modules in the EOBJ object language |
| 1 | `NINDEX` | 2 |
| 4 | `SANEID` | 233579905, the check value of format 3 |
| 8 | `MAJORID`, `MINORID` | 3, 0 |
| 12 | `LBRVER` | the librarian that created it, ASCIC in 32 bytes |
| 44 | `CREDAT`, `UPDTIM` | creation and last update, VMS time |
| 60 | `MHDUSZ` | 33: module headers are 16 + 33 bytes |
| 76 | `NEXTRFA`, `NEXTVBN` | the block after the last one |
| 94 | `HIPREAL`, `HIPRUSD` | the last index block |
| 102 | `IDXBLKS` | index blocks |
| 106 | `IDXCNT` | index entries: modules plus symbols |
| 110 | `MODCNT` | modules |
| 116 | `MODHDRS` | modules |
| 196 | index descriptors | module names, then global symbols |

The other fields are 0. VMS time counts 100 ns units from 17-Nov-1858.

An index descriptor (`IDD$`) is 8 bytes: flags (2 bytes: 0x1D, ASCII keys of
variable length, stored and compared as they are), the longest key allowed (2
bytes: 128) and the root block of the index (4 bytes, 0 if it's empty).

**No change:** vaxpunk could have given ARM64 libraries a type of their own,
but type 7 already says what matters, that the modules are EOBJ records; their
module headers name the architecture.

## Indexes

Both indexes are B-trees of 512-byte blocks. A block holds the number of entry
bytes used (2 bytes), its parent block (4), 6 bytes of fill, then up to 500
bytes of entries. An entry is an RFA, a key length (1 byte) and the key, with
no padding. Keys are in ASCII order.

- In a leaf block, an entry points at a module's data.
- In a block above the leaves, an entry's RFA is a block below with offset
  0xFFFF, and its key is that block's last key.

The module name index has an entry per module. The symbol index has an entry
per global symbol, pointing at the module that defines it. Only strong
definitions go in, as on VMS: the linker takes a module from a library for its
strong definitions, never its weak ones.

## Module data

A module's data is a chain of data blocks. Each holds the number of records
that start in it (1 byte), fill (1), the next block of the chain (4, 0 in the
last), then 506 bytes of data. The module's index entries point at offset 6 of
its first block. The data is a sequence of records, running on from block to
block. A record is a length word, the bytes, and a pad byte if the length is
odd:

1. the module header, below;
2. the module's object records, one each;
3. the end marker: the 3 bytes `77 00 77`.

Module header (`MHD$`), 49 bytes:

| Offset | Field | vaxpunk value |
| --- | --- | --- |
| 0 | `LBRFLAG` | 0 |
| 1 | `ID` | 0xAD |
| 4 | `REFCNT` | index entries for the module: 1 plus its symbols |
| 8 | `DATIM` | insertion time, VMS time |
| 16 | `OBJSTAT` | 0 |
| 17 | `OBJIDLNG`, `OBJID` | the module's ident, ASCIC in 32 bytes |
