# PRD-0008 — RMS: every record format, relative files and indexed files

Oct 6, 2026 · @Marko Mikulicic

## Context and goal

RMS today, `rms.mar`, is sequential files and little else. `$GET` reads
`VAR` and `FIX` records and says `RMS$_RFM` to the rest; `$PUT` appends;
there is one stream per file, no `$UPDATE`, `$DELETE`, `$FIND` or
`$TRUNCATE`, no access by RFA, no block I/O, and `$CREATE` takes no XAB but
the protection one. [PRD-0003](0003-multi-user-vms.md) lists relative and
indexed files as a non-goal of Milestone 1 and these gaps as its backlog
item 23.

That holds as long as every program is ours. It stops holding with the
first program from outside. On VMS indexed files are everywhere: the UAF,
the rights database, the queue file, mail folders, `NETPROXY`, and most
applications ever written for VMS, which keep their data in them and read
it by key. `CONVERT`, `ANALYZE/RMS_FILE` and FDL are how a VMS user makes,
reorganizes and checks such files, and DCL's `READ/KEY` how a procedure
reads them.

The goal is **RMS as a VMS programmer knows it**: every record format of
sequential files, relative files, and indexed files with primary and
alternate keys, through the same FAB, RAB and XAB calls, bit for bit the
same on disk as VMS's, so a file made by `CONVERT` on OpenVMS reads on
vaxpunk and one vaxpunk writes passes `ANALYZE/RMS_FILE/CHECK` on OpenVMS.

Goals:

- **Sequential files, complete.** `VFC`, `STM`, `STMLF`, `STMCR` and `UDF`
  records next to `VAR` and `FIX`; the record attributes `CR`, `FTN`, `PRN`
  and `BLK`; `$FIND`, `$UPDATE`, `$TRUNCATE`, `$REWIND`, `$FLUSH`, access
  by RFA; block I/O with `$READ`, `$WRITE` and `$SPACE`.
- **Relative files.** Fixed cells numbered from 1, a record read, written
  or deleted by its relative record number, or in order.
- **Indexed files.** Prologue 3: a primary key and up to 254 alternate
  keys, segmented keys, duplicates where the key allows them, key changes
  on `$UPDATE` where it allows them; `$GET` and `$FIND` by key, exact,
  generic, greater or equal, greater; sequential reads along any key;
  bucket splits, RRVs, and key, index and data compression.
- **The XABs** a program uses to make and look at such files: `XABKEY`,
  `XABALL`, `XABSUM`, `XABFHC`, `XABDAT`, `XABRDT`, with `XABPRO` as now;
  and `$DISPLAY`.
- **The utilities:** `CREATE/FDL`, `CONVERT`, `ANALYZE/RMS_FILE`, and the
  callable `FDL$CREATE` and `FDL$PARSE`.
- **DCL:** `OPEN` of any organization, `READ/KEY` and `/INDEX`,
  `READ/DELETE`, `WRITE/UPDATE`; `DIRECTORY/FULL` shows a file's
  organization and keys; `COPY` of an indexed file keeps it whole.
- **Shared files** once PRD-0003's lock manager is in: bucket and record
  locks, `RMS$_RLK`, `$FREE` and `$RELEASE`, several streams on one file.

Decisions this PRD rests on:

| Decision | Choice |
| --- | --- |
| On-disk format | VMS's: prologue 3 indexed files, VMS's relative files, the record attributes in the Files-11 header as now. No format of our own |
| Language | MACRO-32, as DEC's own RMS was, on the calling standard of [PRD-0005](0005-macro32-on-the-calling-standard.md). Not waiting for BLISS-64 |
| Where it runs | With the rest of RMS, in the executive in kernel mode, until backlog item 23 moves RMS to executive mode; nothing here makes that move harder |
| Modules | Split by organization: the FAB and name code `rms.mar` has now, then sequential, relative and indexed, so each can be read alone |
| Spec | *OpenVMS Record Management Services Reference Manual*, *Guide to OpenVMS File Applications*, *OpenVMS Record Management Utilities Reference Manual* |
| Oracle | OpenVMS Alpha V8.4-2L1 in AXPbox: its `CONVERT` makes files we read, its `ANALYZE/RMS_FILE/CHECK` judges files we write, and the same test program runs on both |
| Second implementation | A reader and checker of relative and indexed files in Rust, in `ods`, for tests on the host |

## Non-goals

- **No copying.** The VAX/VMS V4.3 sources' RMS modules and definitions,
  and the OpenVMS images, are read for reference only, as AGENTS.md says
  of FreeVMS. The structure layouts come from the manuals and from
  `ANALYZE/RMS_FILE` dumps of the oracle's files.
- **Prologue 1 and 2 indexed files.** VMS still opens them; we refuse them
  with `RMS$_PLV`. `CONVERT` on OpenVMS turns them into prologue 3.
- **Collated keys and packed decimal keys** at first. String, signed and
  unsigned integer of 2, 4 and 8 bytes, and their descending forms, are in.
- **RMS Journaling**: after-image, before-image and recovery units. No
  `SET FILE/AI_JOURNALING`, no DECdtm.
- **Global buffers**, `SET FILE/GLOBAL_BUFFERS`, and deferred write across
  processes.
- **Remote files.** No DAP or FAL; `NODE::` stays [PRD-0002](0002-networking.md)'s.
- **`EDIT/FDL`**, the interactive FDL editor, and `ANALYZE/RMS_FILE`'s
  interactive mode. FDL files are written by hand or by
  `ANALYZE/RMS_FILE/FDL`.
- **SORT/MERGE.** `CONVERT` sorts the records it loads itself, in memory
  where they fit; a `SORT` utility is another PRD.
- **ASTs and completion routines** on record services stay backlog item 23:
  every service here is synchronous, as now.

## Record formats and sequential files

The formats `crosstools/ods/docs/records.md` describes, read and written:

- `FIX`, `VAR` and `VFC` with `RAB$L_RHB` for the fixed part; `STM`,
  `STMLF` and `STMCR`, where `$PUT` adds the terminator and `$GET` strips
  it; `UDF`, block I/O only.
- `BLK`: no record crosses a block, as `$PUT` already lays out `VAR`
  records; checked for every format that allows it.
- `CR`, `FTN` and `PRN` are kept and given back; RMS doesn't apply them.
  `TYPE` and the terminal driver do, as now.
- `$FIND` and `$GET` set `RAB$W_RFA`; `RAB$B_RAC` = `RAB$C_RFA` goes back to
  a record; `$UPDATE` rewrites the current record at its size;
  `$TRUNCATE` cuts the file after it; `$REWIND` goes back to the start;
  `RAB$V_EOF` at `$CONNECT` starts at the end; `$PUT` in the middle of a
  file with `RAB$V_TPT` truncates after the record.
- Block I/O: `FAB$V_BIO` or `BRO` at `$OPEN`, then `$READ`, `$WRITE` and
  `$SPACE` by VBN. `COPY` uses it for files that aren't sequential, as
  VMS's does, so an indexed file copies whole.

## Relative files

A prologue block, then buckets of fixed cells: a control byte that says
whether the cell holds a record and whether it was deleted, then the
record, `FAB$W_MRS` bytes, or a `VAR` record up to that size. `FAB$L_MRN`
bounds the record number. `$PUT` by key writes cell n, or `RMS$_REX` if it
holds a record; `$PUT` in order writes the next; `$GET` and `$FIND` by key
or in order, skipping empty cells; `$DELETE` empties a cell; `$UPDATE`
rewrites it. The relative record number comes and goes in the key buffer,
`RAB$L_KBF`, and in `RAB$L_BKT` for sequential access.

## Indexed files

Prologue 3, as `CONVERT` on OpenVMS makes it by default:

- **Prologue.** Key descriptors, one per key, chained from VBN 1, and
  area descriptors after them: each area's bucket size, extents and
  allocation.
- **Primary key.** An index of buckets down to the data buckets, which
  hold the records in primary key order, each with its control byte,
  record ID and key, then its data, compressed if the key says so.
- **Splits.** A full data bucket splits in two or three; records that move
  leave a record reference vector, RRV, so an RFA and every alternate
  index entry still find them.
- **Alternate keys.** Their own index, down to SIDR buckets: each key value
  with the RFAs of the records that have it.
- **Compression.** Front compression of keys in data and index buckets,
  rear truncation of index keys, and data compression, each where the key
  descriptor asks; read and written, since the oracle's files have them.
- **Services.** `$PUT` inserts by primary key, `RMS$_DUP` on a duplicate it
  doesn't allow, `RMS$_OK_DUP` on one it does; `RAB$V_UIF` updates instead.
  `$GET` and `$FIND` by key of reference `RAB$B_KRF`, with `RAB$V_KGE`,
  `RAB$V_KGT` and a short `RAB$B_KSZ` for generic; `RMS$_RNF` when there is
  none. Sequential access follows the current key. `$UPDATE` may change
  alternate keys marked `XAB$V_CHG`, else `RMS$_CHG`; `$DELETE` removes
  the record and its alternate entries. `$CREATE` takes the keys from the
  `XABKEY` chain and the areas from the `XABALL`s.
- **Limits.** The prologue's: 255 keys, 8 segments, 255-byte keys, 63-block
  buckets. Ours, set by pool (*Open questions*): fewer buffers per stream
  than VMS's defaults, and bucket sizes past what fits refused with
  `RMS$_BKS`.

Every condition value is VMS's, in `starlet.mlb`'s `$RMSDEF`, with VMS's
message text.

## Utilities and DCL

- `CREATE/FDL=file [name]` makes an empty file as an FDL file describes
  it. `FDL$PARSE` reads FDL into a FAB, RAB and XAB chain; `FDL$CREATE`
  does that and `$CREATE`s.
- `CONVERT in out` with `/FDL`, `/APPEND`, `/MERGE`, `/CREATE`,
  `/EXCEPTIONS_FILE`, `/FIXED_CONTROL`, `/PAD`, `/TRUNCATE`, `/SORT`,
  `/STATISTICS`; loads an indexed file in primary key order, filling
  buckets as the FDL says, and builds the alternate indexes after.
  `CONVERT/RECLAIM` frees the empty buckets of a prologue 3 file.
- `ANALYZE/RMS_FILE` with `/CHECK`, `/FDL` and `/STATISTICS`, its report
  laid out as VMS's.
- DCL's `OPEN` opens any organization; `READ/KEY=/INDEX=/MATCH=` reads by
  key, `READ/DELETE` deletes what it read, `WRITE/UPDATE` rewrites it.
  These need PRD-0003's step 8 first.
- `DIRECTORY/FULL` says `File organization: Indexed, Prologue: 3, Using 2
  keys` and the rest, as VMS does. `TYPE` of an indexed file reads it by
  primary key.

## Sharing

After PRD-0003's step 7 puts files on the lock manager. `FAB$B_SHR` with
`SHRGET`, `SHRPUT`, `SHRUPD` and `SHRDEL` on relative and indexed files;
a lock on each bucket while a service works in it and on each record a
stream holds; `RMS$_RLK` when another stream holds it, `RAB$V_WAT` to wait
instead, `RAB$V_NLK` and `RAB$V_RRL` to read past it, `RAB$V_ULK` with
`$FREE` and `$RELEASE`. Several RABs connected to one FAB with
`FAB$V_MSE`.

## Testing strategy

- **The same program on both.** `RMSTEST.MAR` makes files of every
  organization and format and works them, printing what each service
  returns and every record it reads. It runs under the MACRO-32 compiler
  on OpenVMS and under vaxpunk; the oracle's output is committed, and the
  boot test compares vaxpunk's to it.
- **Files across.** Fixtures made on the oracle with `CREATE/FDL` and
  `CONVERT`, a few of each shape (one key, many keys, duplicates,
  compression, many levels), are committed under `crosstools/ods/fixtures` with their
  `ANALYZE/RMS_FILE/FDL` and a dump by every key. vaxpunk reads them and
  must print the same dump. Going the other way, files `RMSTEST` writes
  are copied out with `ods` and checked by `ANALYZE/RMS_FILE/CHECK` on the
  oracle, by hand and before each step closes; CI never needs AXPbox.
- **On the host.** The `ods` reader dumps and checks the same fixtures in
  `cargo test`, and `crosstools/ods/fuzz` fuzzes it, so a damaged file is an error,
  not a crash.
- **Random operations.** A host program generates a long run of `$PUT`,
  `$UPDATE`, `$DELETE` and keyed `$GET`s with a seed, and the result each
  should have from a `BTreeMap`; vaxpunk runs it, and after it the `ods`
  checker walks the file. Splits, RRVs and SIDR chains get exercised the
  way hand-written tests don't.

## Open questions

- [ ] **Buffers.** VMS gives each stream several buffers of a bucket's size
  each, up to 32 KB a bucket, in the process's P1. Ours come from 512 KB
  of nonpaged pool shared by everything. Smaller defaults and a total cap,
  or move RMS's buffers to P1 now, ahead of executive mode?
- [ ] **Files-11 first.** An indexed file extends often and in small
  pieces, and a header without extension headers runs out of map
  pointers; at 4,096 blocks a disk holds few big files. Backlog item 22's
  extension headers come before step 6, or `CONVERT` allocates contiguous
  best try and that is enough for a while?
- [x] **`SYSUAF.DAT`.** Indexed or fixed records? Indexed, as on
  OpenVMS Alpha V8.4, whose `SYSUAF.DAT` has `VAR` records up to 1,412
  bytes in 3-block buckets and four keys: the username, a 32-byte string
  at offset 4; the UIC, 4 bytes at 36, with duplicates; and two 8-byte
  binary keys at 36 and 44, with duplicates. Ours has the same keys, so
  PRD-0003's step 4 waits for steps 1 to 6 here.
- [x] **The oracle's MACRO-32.** Is the MACRO-32 compiler on the
  installed playground system? Yes: it is part of the base OpenVMS Alpha
  install, `SYS$SYSTEM:MACRO.EXE` (AMAC V5.0-120-4) with `MACRO.CLD`,
  `LINK`, `STARLET.MLB` and `STARLET.OLB`, so `MACRO/MIGRATION` builds
  `RMSTEST.MAR` there. `CONVERT`, `ANALYZE/RMS_FILE` and `CREATE/FDL` are
  installed too.

## Work order

Each step ends in something you can run or look at.

Steps 1 to 7 are done, and of step 8 `DIRECTORY/FULL`, `TYPE` and `COPY`
of relative and indexed files: what is left of it, DCL's `OPEN`, `READ`
and `WRITE`, comes with PRD-0003's step 8, and step 9 with its step 7.
`image/tests/rms.rs` checks each against the fixtures and a model, and
the reports `ANALYZE/RMS_FILE` and `DIRECTORY/FULL` write matched the
oracle's on every fixture.

1. **Layouts.** `crosstools/ods/docs/indexed.md` and `crosstools/ods/docs/relative.md`: the
   prologue, area and key descriptors, buckets, records, RRVs and SIDRs,
   from the manuals and `ANALYZE/RMS_FILE/INTERACTIVE` on the oracle; the
   oracle fixtures with their dumps. *Visible:* the notes and fixtures.
2. **Host reader and loader.** `ods` reads relative and indexed files by
   any key and checks them, and loads records into a new indexed file,
   which is how the build writes `SYSUAF.DAT`. *Visible:* `ods records
   disk.img FILE --key 1` prints the oracle's dump; `ods check-file`
   reports a corrupted fixture; a loaded UAF passes
   `ANALYZE/RMS_FILE/CHECK` on the oracle.
3. **Sequential, complete.** Every record format and attribute, `$FIND`,
   `$UPDATE`, `$TRUNCATE`, `$REWIND`, RFA access, block I/O, `XABFHC`,
   `XABDAT`, `XABALL` and `XABSUM` on `$OPEN` and `$DISPLAY`; `rms.mar`
   split by organization. *Visible:* `RMSTEST`'s sequential part matches
   the oracle; `COPY` of a `STMLF` file keeps its format.
4. **Relative files.** *Visible:* `RMSTEST`'s relative part matches; its
   file passes `ANALYZE/RMS_FILE/CHECK`.
5. **Indexed, read.** `$OPEN` of a prologue 3 file, `$GET` and `$FIND` by
   any key, sequential along any key, RFA. *Visible:* vaxpunk prints the
   same dumps as the oracle for every fixture.
6. **Indexed, write.** `$CREATE` with `XABKEY` and `XABALL`, `$PUT` with
   splits and RRVs, `$UPDATE`, `$DELETE`, compression. *Visible:* the
   random run passes the `ods` checker, and its file passes
   `ANALYZE/RMS_FILE/CHECK` on the oracle.
7. **FDL, `CONVERT`, `ANALYZE/RMS_FILE`.** *Visible:* `CONVERT/FDL` of a
   text file into an indexed file in the boot test, then
   `ANALYZE/RMS_FILE/CHECK` on vaxpunk says it is clean.
8. **DCL.** `OPEN`, `READ/KEY`, `READ/DELETE`, `WRITE/UPDATE`,
   `DIRECTORY/FULL`, `TYPE`. Needs PRD-0003's step 8. *Visible:* a
   procedure looks a name up in an indexed file and updates its record.
9. **Sharing.** Bucket and record locks, `$FREE`, `$RELEASE`, several
   streams. Needs PRD-0003's step 7. *Visible:* two sessions update
   different records of one file; on the same record the second sees
   `%RMS-E-RLK`.

Steps 1 to 6 come before PRD-0003's step 4, whose `SYSUAF.DAT`,
`AUTHORIZE` and `LOGINOUT` use indexed files; the rest can come after.
