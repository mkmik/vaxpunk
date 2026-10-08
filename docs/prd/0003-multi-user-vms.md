# PRD-0003 — Milestone 1: a multi-user VMS you log in to

Oct 5, 2026 · @Marko Mikulicic

## Context and goal

Milestone 0 is done: seL4 boots the PAL, the PAL starts the executive, and
the executive runs processes with ASTs, `$QIO` and drivers, mailboxes, a
writable Files-11 and RMS, DCL with CLD tables and command procedures, EDT,
and TCP/IP with `SET HOST` ([PRD-0002](0002-networking.md)). It was built
one feature at a time, each leaving a `ponytail:` note where it cut a
corner. There are about 140 of them, and nothing yet says which matter.

Every process is still `SYSTEM`. Any process may write any block of any
disk, call `$CMKRNL`, or delete an open file. That is fine for one user at
the console, and it stops being fine the moment a second one logs in over
`SET HOST`.

Milestone 1 is **a multi-user VMS you log in to**: several users, each with
a username, a password, a UIC and privileges, log in at the console or over
the network, own their files, can't touch each other's, submit batch jobs,
and share files through a lock manager. A VMS user from 1990 should find
nothing missing from an ordinary day that isn't listed under *Non-goals*.

Goals:

- `Username:` and `Password:` at the console and over `SET HOST`, checked
  against `SYSUAF.DAT`, which `AUTHORIZE` maintains.
- Each process has a UIC and privileges, and the executive checks them:
  services that need a privilege say `SS$_NOPRIV` without it, files and
  devices have an owner and a protection mask, and `SS$_NOPRIV` stops the
  wrong user.
- `$ENQ` and `$DEQ`: a lock manager the file system and RMS use, so two
  processes can share a file and one volume's work doesn't wait on another's.
- `SUBMIT`, `PRINT` and `SHOW QUEUE`: a job controller that runs command
  procedures in batch processes and writes a log file.
- The command language a batch job needs: block `IF`, `ON`, and the common
  lexical functions.
- This PRD replaces the `ponytail:` notes as the list of what's left: the
  backlog below ranks them, and a note that isn't in it is fine as it is.

Decisions this PRD rests on:

| Decision | Reason |
| --- | --- |
| Security before features | Each service written without privilege checks is one more to go back to; the checks are cheapest now |
| VMS's data layouts (`$UAFDEF`, `$LKBDEF`, `$PRVDEF`, protection masks) | Programs and habits carry over; the layouts are documented and stable |
| One CPU, no cluster | ADR-0003 holds; a lock manager for one node is a fraction of the work and the interface is the same |
| Passwords hashed with a modern function, stored in `$UAFDEF`'s field | Purdy's polynomial is broken; nothing reads our UAF but us |
| Native code stays MACRO-32 | No BLISS or C compiler yet; nothing here needs one |

## Non-goals

- Clusters, the distributed lock manager's remote side, SMP.
- Paging to a page file, swapping, working set trimming. Memory is
  allocated and kept, as now.
- Access control lists, security auditing, `SET AUDIT`. UIC protection only.
- Installed images, `INSTALL`, and processes sharing a shareable image's
  pages. They come after Milestone 1 (*Backlog*, tier 3); shareable
  images themselves are there, a copy in each process
  ([ADR-0028](../adr/0028-shareable-images.md)).
- RMS relative files, and indexed files beyond what `SYSUAF.DAT` needs:
  [PRD-0008](0008-rms-record-and-indexed-files.md).
- DECnet, mail and remote file access ([PRD-0002](0002-networking.md)
  keeps those).
- Running VAX or Alpha binaries.

## Requirements

### Login

- `LOGINOUT.EXE` runs in a new process on a terminal: it prompts for a
  username and password, reads `SYS$SYSTEM:SYSUAF.DAT`, and on success sets
  the process's name, UIC, privileges, quotas, default device and
  directory, `SYS$LOGIN`, `SYS$SCRATCH` and its command interpreter
  (closing ADR-0006's note), then runs `SYS$MANAGER:SYLOGIN.COM` and the
  user's `LOGIN.COM`.
- Three failures end the connection. A wrong username is told apart from a
  wrong password only in the UAF's failure count, never on the screen.
- `LOGOUT` prints VMS's `logged out at` line; the process is deleted and
  the terminal is free for the next `Username:`.
- `SYSTARTUP_VMS.COM` runs in a `STARTUP` process before anyone logs in, as
  VMS does (closing DESIGN-0002's note), and the console then waits for a
  login instead of starting `SYSTEM`'s DCL.
- `AUTHORIZE` (`MCR AUTHORIZE` or `RUN SYS$SYSTEM:AUTHORIZE`) with `ADD`,
  `MODIFY`, `REMOVE`, `SHOW` and `LIST`, and `/PASSWORD`, `/UIC`,
  `/PRIVILEGES`, `/DEFPRIVILEGES`, `/DEVICE`, `/DIRECTORY`, and quotas.
  `SET PASSWORD` changes one's own.
- `SYSUAF.DAT` is an indexed file, as on OpenVMS Alpha V8.4: `$UAFDEF`
  records keyed by username and by UIC, read and written through RMS
  ([PRD-0008](0008-rms-record-and-indexed-files.md)).
- The build makes a UAF with `SYSTEM` and `DEFAULT` records, written on the
  host by `ods`; the password for `SYSTEM` is `MANAGER`, and the README
  says to change it.

### Terminals

- Each terminal is a UCB of its own, with its line being read, its typeahead,
  its CTRL/C and CTRL/Y ASTs and its characteristics, rather than
  `ttdriver.mar`'s one console.
- A `SET HOST` connection is a terminal, `TNAnn:`, that `TELNETD` makes for
  `LOGINOUT`, not a bare `BGnn:` unit; EDIT and every other program see it
  as they see the console.
- `IO$_SETMODE` and `IO$_SENSEMODE` with the characteristics the common
  programs use: width, page, `ECHO`, `NOBRDCST`, `ESCAPE`, `PASTHRU`.
  `SET TERMINAL` and `SHOW TERMINAL`.
- `$BRKTHRU`, for `REPLY` and the job controller's messages.

### Privileges, UICs and protection

- A process has current, authorized and image privileges (`$PRVDEF`), a UIC
  and a rights list of its UIC alone. `$SETPRV` (now a stub) and `SET
  PROCESS/PRIVILEGES` change them within what the UAF authorizes.
- The executive checks the privileges VMS does for the services that exist:
  `CMKRNL`, `CMEXEC`, `SETPRV`, `WORLD` and `GROUP` (other processes in
  `$DELPRC`, `$FORCEX`, `$GETJPI`, `$SUSPND`), `PRMMBX` and `TMPMBX`,
  `SYSNAM` and `GRPNAM` (logical name tables), `LOG_IO` and `PHY_IO`
  (closing `f11.mar`'s note), `MOUNT` and `VOLPRO`, `OPER`, `SYSPRV`,
  `BYPASS`, `READALL`.
- Files have an owner UIC and a protection mask in their header, which
  Files-11 already has room for. RMS checks them on `$OPEN`, `$CREATE`
  and `$ERASE`; directories are checked for the lookup. `SET PROTECTION`,
  `SET FILE/OWNER`, and `DIRECTORY/OWNER/PROTECTION`.
- Volumes and devices have an owner and a protection mask too: `MOUNT`
  and `INITIALIZE` take `/OWNER` and `/PROTECTION`; a mailbox has its
  creator's.
- Logical name tables `LNM$GROUP` and `LNM$JOB`, each logged-in job's.

### Quotas

- The UAF's `BYTLM`, `BIOLM`, `DIOLM`, `ASTLM`, `TQELM`, `PRCLM`, `FILLM`
  and `ENQLM` are charged and checked, so one user can't take the pool.
- `$GETJPI` returns them, and CPU time and I/O counts, which the scheduler
  and `$QIO` now keep. `SHOW PROCESS/QUOTAS` and `/ACCOUNTING`.

### Lock manager

- `$ENQ`, `$ENQW`, `$DEQ` and `$GETLKI` on a resource tree with the six
  lock modes, conversions, value blocks, the `NOQUEUE`, `CONVERT`,
  `SYSTEM` and `VALBLK` flags, completion and blocking ASTs, and
  deadlock detection that fails one request with `SS$_DEADLOCK`.
- Resource names are qualified by access mode and UIC group, as on VMS.
- The file system takes a lock per volume and per file, in place of
  `FIL$LOCK` (closing ADR-0020's notes), and the holder's priority is no
  longer a problem because two volumes no longer wait on each other.
- RMS shares a file between processes: `FAB$B_SHR` with `SHRGET`,
  `SHRPUT`, `UPI`; `RMS$_FLK` when it can't. `$ERASE` of an open file
  says so (closing `rms.mar`'s note).
- `SHOW SYSTEM` and `SHOW PROCESS/LOCKS` see them.

### Batch and print

- A job controller process, `JOB_CONTROL`, keeps the queues in
  `SYS$SYSTEM:QMAN$MASTER.DAT` and takes requests with `$SNDJBC` and
  `$GETQUI`.
- `INITIALIZE/QUEUE/BATCH`, `START/QUEUE`, `STOP/QUEUE`, `DELETE/QUEUE`.
- `SUBMIT file` runs it in a new batch process under the submitter's
  username, with `SYS$OUTPUT` a log file in `SYS$LOGIN`; `/AFTER`,
  `/HOLD`, `/PARAMETERS`, `/LOG`, `/NOTIFY`. `SHOW QUEUE`, `DELETE/ENTRY`,
  `SYNCHRONIZE`.
- `PRINT` goes to a print queue whose symbiont writes to a terminal or a
  file. One symbiont, no forms or device control libraries.
- `SHOW TIME` and `$ASCTIM`/`$BINTIM`, which `/AFTER` and the log file need.

### Command language

- Block `IF`/`THEN`/`ELSE`/`ENDIF`, `ON ERROR`/`WARNING`/`CONTROL_Y`,
  `SET NOON`, `$STATUS` and `$SEVERITY`.
- Lexical functions: `F$GETJPI`, `F$GETSYI`, `F$GETDVI`, `F$TRNLNM`,
  `F$SEARCH`, `F$PARSE`, `F$ENVIRONMENT`, `F$MODE`, `F$TIME`, `F$LENGTH`,
  `F$EXTRACT`, `F$LOCATE`, `F$ELEMENT`, `F$EDIT`, `F$INTEGER`,
  `F$STRING`, `F$FAO`, `F$VERIFY`.
- `OPEN`, `READ`, `WRITE` and `CLOSE` on files, and `SET VERIFY`.
- Input lines without `$` go to the running image as `SYS$INPUT`, as VMS
  does, rather than being skipped.

## Backlog

The `ponytail:` notes, ranked. Each line says where they are and what
closes them. **Tier 1** blocks Milestone 1 and is part of its work order.
**Tier 2** can take the system down or lose data, and is fixed when the
code near it is next touched or before Milestone 1 ends, whichever is
first. **Tier 3** is fidelity: what VMS does that we don't yet, worth doing
when a user or program needs it. **Tier 4** is scaffolding that is fine
until the hardware or size changes; leave it.

### Tier 1: Milestone 1

| # | Notes | What closes them |
| --- | --- | --- |
| 1 | Privileges done (step 1), files' and volumes' protection (step 2); left: mailboxes with no protection mask (ADR-0019, `mbdriver.mar`); a `$CREMBX` logical name takes `SYSNAM` until `LNM$JOB` (`mbdriver.mar`) | *Privileges, UICs and protection* |
| 2 | No username or password: `TELNETD` logs in as `SYSTEM` (`telnetd.mar`, DESIGN-0003); the link address picks the command interpreter (ADR-0006); `SYS$DISK` and `SYSTARTUP_VMS.COM` set up by DCL, not `LOGINOUT` and `STARTUP` (DESIGN-0002) | *Login* |
| 3 | Terminals per UCB done, with `TNAn:` for `SET HOST` (step 3, ADR-0027); left: no `$BRKTHRU`; a hangup ends the terminal's reads and writes, not its process; at most 9 `TNAn:` units (`ttdriver.mar`, DESIGN-0002 *I/O*) | *Terminals*, and `LOGINOUT` for the hangup |
| 4 | No quotas: `BIOLM`, `DIOLM`, `BYTLM` (ADR-0013), `BUFQUO` (ADR-0019), ASTs (DESIGN-0002 *ASTs*); `$GETJPI` has no CPU times, quotas or counts (`getjpi.mar`, `show.mar`) | *Quotas* |
| 5 | One file system lock for every volume, no per-file lock, no priority boost (ADR-0020, `f11.mar`, DESIGN-0002 *Files*); no file sharing or locking, `$ERASE` deletes an open file (ADR-0009, `rms.mar`) | *Lock manager* |
| 6 | DCL procedures: no `ON`, block `IF`, lexical functions, `OPEN` or `READ`; lines without `$` skipped (`dcl.mar`) | *Command language* |
| 7 | `$ASCTIM`, `$NUMTIM` (PRD-0004's pilot, `numtim.b64`) and the date in `SHOW PROCESS` and `SHOW SYSTEM` done; left: `$BINTIM`, `SHOW TIME` | *Batch and print* |
| 8 | Owners and protection done (step 2); left: a new file has the default protection, not the process's (`f11wrt.mar`), which `LOGINOUT` sets from the UAF; no version limit (`rms.mar`'s `$CREATE_DIR`) | *Privileges, UICs and protection* |
| 9 | Terminals, `TNAn:` too, listed from their UCBs (step 3); left: `BGA0:` and its units aren't seen by `$DEVICE_SCAN` and `$GETDVI`; a VCB stands for a disk in `$GETDVI` rather than its UCB (`netdriver.mar`, `getdvi.mar`, DESIGN-0002 *Devices*) | Every device listed from its UCB |

### Tier 2: can crash or lose data

| # | Notes | What closes them |
| --- | --- | --- |
| 10 | `IOC$POST` writes the caller's buffer and IOSB without probing them: deleting the pages under a pending read takes the system down (ADR-0013, `qio.mar`) | Probe in `IOC$POST`, or lock the pages (`$LCKPAG` is a stub) |
| 11 | No write ordering or flush on `DKB0:`: a host crash mid-`COPY` can leave a header without its directory entry (ADR-0012); no backup home block or index file header (`f11wrt.mar`, DESIGN-0002) | Write the header before the directory entry, a flush on `DISMOUNT`, and the backup home block `INITIALIZE` writes |
| 12 | A fault in an outer mode ends the image past its exit handlers (`process.mar`, DESIGN-0002) | A last chance handler that calls `$EXIT` |
| 13 | An AST routine's kernel code can be deleted halfway without `IPL$_ASTDEL` around it (ADR-0011) | Raise IPL as VMS does around such code |
| 14 | A network connection whose `CLOSE` finds no pool stays open; unit numbers wrap at 99 whether in use or not (`netdriver.mar`) | Preallocate the close message in the UCB; skip units in use |
| 15 | A name whose versions spill into a second directory record matches twice (`f11.mar` `FIL$SEARCHDIR`); a directory is rewritten whole on each change (`f11wrt.mar`) | Search every record of a name; it matters once a file has many versions |
| 16 | `EXTZV`/`INSV` read and write 8 bytes around the field, not atomically, and fault across a page end (`vmacro/src/insn.rs`); `crosstools/vtools/tests/macro32-bugs/fields.mar` shows it | Narrower accesses at a page end; atomic where the field is shared |
| 17 | Shift counts of 32 and up wrap instead of clearing (`vmacro/src/insn.rs`); `crosstools/vtools/tests/macro32-bugs/shifts.mar` shows it | A compare and select, as the note says |
| 18 | `IOC$CANCEL` doesn't wait for rundown's cancels to finish (`qio.mar`) | Needed once a driver can't stop at once: the interrupt-driven disk (tier 4) |
| 19 | Common event flag clusters are kept while processes wait on them (`event.mar`) | Fine as is; listed so it isn't fixed twice |

### Tier 3: fidelity

| # | Notes | What closes them |
| --- | --- | --- |
| 20 | Shareable images are mapped a copy in each process, as VMS maps one that isn't installed, and only from P0 images, without `DATA` entries, `GSMATCH` or one calling another; `LIBRTL.EXE` has the `LIB$` routines but `LIB$GET_FOREIGN`, and the CLI parser is linked into every image (ADR-0017, ADR-0028); images run where they are linked (`process.mar`, `pal/src/main.c`); `$CRMPSC`, `$MGBLSC` are stubs; one mapping per PFN (DESIGN-0001, `main.c`) | A frame capability per extra mapping, global sections, then `INSTALL /SHARED` |
| 21 | An image is read whole when activated (`f11.mar` `FIL$OPENFILE`) | Map sections page by page, with global sections |
| 22 | Files-11: one index file bitmap block, so at most 4,096 files; no extension headers; free blocks counted each time; the index file grows 16 blocks at a time (ADR-0009, ADR-0012, `f11.mar`, `f11wrt.mar`, DESIGN-0002) | Extension headers when a file's map fills; a free count in the VCB |
| 23 | RMS: one stream per file, no wildcard directories, ASTs or completion routines; `SET PROTECTION` and `DIRECTORY/PROTECTION` need read access to the file, where VMS's ask the XQP; runs in kernel mode (`rms.mar`, `set.mar`, `directory.mar`, DESIGN-0002) | Sharing with PRD-0008's step 9; wildcard directories with item 24; executive mode with the mode threads of ADR-0005 |
| 24 | Utilities take one file, no wildcards (`copy.mar`, `type.mar`, `delete.mar`, `directory.mar`); no qualifiers on `MOUNT`, `DISMOUNT`, `INITIALIZE` but `/OWNER_UIC` and `/PROTECTION` (`mount.mar`, `dismount.mar`, `init.mar`); `SHOW DEVICES` without `/FULL` (`dcl.mar`) | Wildcards through `$SEARCH` in each; qualifiers as Milestone 1 needs them |
| 25 | Logical names: no access modes, attributes, tables of one's own, directories, or rooted or concealed names; `$DELLNM` without a name; DCL walks the tables itself for `SHOW LOGICAL`; `SYS$SYSROOT` is a search list of whole volumes, `MDA0:` and `DKA0:`, not of rooted directories (`lnm.mar`, `dcl.mar`, DESIGN-0002) | Access modes first, which `LOGINOUT`'s user-mode names need; `LNM$GROUP` and `LNM$JOB` are tier 1 |
| 26 | One message table, no message files, `$PUTMSG` or `LIB$SIGNAL` (`getmsg.mar`, `cli.mar`, `dcl.mar`, DESIGN-0002) | Message files when a utility brings its own facility; `$PUTMSG` with condition handling |
| 27 | `HELP` has no text (`help.mar`) | A help library, `HELPLIB.HLB` |
| 28 | Mailbox writes never wait, no `IO$_WRITEOF` or attention ASTs (`mbdriver.mar`) | When a program needs them; the job controller may |
| 29 | Terminal editing and `SET HOST` done (step 3); left: the recall buffer is the terminal's, not DCL's, and there's no `RECALL`; no CTRL/O, CTRL/T, XON and XOFF or formatted writes (`ttdriver.mar`, DESIGN-0002); no `TELNET>` command mode at CTRL/] (`rtpad.mar`); EDIT lacks several commands (`edit.mar`) | DCL's own recall buffer with `RECALL`; the rest when a user misses it |
| 30 | `$EXPREG` grows P0 only (`memory.mar`); image rundown frees every P0 page, mapped or not (`process.mar`) | When a program grows P1 or the walk shows in a profile |
| 31 | The scheduler has no priority boosts or decay (`sched.mar`, DESIGN-0002) | Boosts on I/O completion and wakes, with decay; worth it once interactive users share the CPU with batch jobs |
| 32 | The CLI: parse limits of 128 entities, 1 KB of values, a 512-byte line; the parse copied to a fixed address; the first `SET COMMAND` error ends the compile; at most 8 `SET COMMAND` files; a qualifier the syntax lacks is an error, not ignored; an entity present by default doesn't count; `$IMGACT` calls the image (ADR-0017, ADR-0018, `cli.mar`, `cdu.mar`, `process.mar`, DESIGN-0002) | When a command or a CLD file runs into one |
| 33 | `$GETDVI` and `$GETJPI` are done at once with no AST (`getdvi.mar`, `getjpi.mar`) | Fine on one node; the `W` and non-`W` forms behave the same |
| 34 | AST routines get their parameter only, not R0, R1, PC and PSL (DESIGN-0002) | When a program reads them |
| 35 | One route and one interface; no buffer lists, socket options, read and write flags or `IO$M_NOW` on sockets (`tcpip.mar`, `netdriver.mar`, DESIGN-0003, ADR-0024) | PRD-0002's work order; the sockets library will want options and flags |
| 36 | `LIB$GET_INPUT`'s fixed-length string isn't blank padded (`getinput.mar`); `MDA0:` has a fixed size (`mddriver.mar`) | When a caller notices; `INITIALIZE/SIZE` once DCL passes qualifiers to it |

### Tier 4: scaffolding

Fine until the hardware, the size or the speed changes. Each note already
says what replaces it; this PRD doesn't schedule them.

| # | Notes |
| --- | --- |
| 37 | Disk I/O polled and synchronous, the CPU waits for each request; the file system doesn't wait on a queued IRP (ADR-0007, ADR-0015, DESIGN-0001, DESIGN-0002, `main.c`) |
| 38 | The console UART polled each 10 ms tick, at QEMU virt's fixed address; device addresses not from the DTB (DESIGN-0001, `main.c`) |
| 39 | 1,024 PFNs, 4 MB, and 96 page tables, as much as the root CNode leaves room for (DESIGN-0001, `main.c`). This one turns into tier 1 when many logged-in users and batch jobs run out of memory: frame caps in a CNode of their own |
| 40 | A seL4 call per page to switch the kernel's view of two processes; page protection set once; the tick a PAL thread (ADR-0004, ADR-0005, `main.c`) |
| 41 | The TCP/IP component's page mapped writable and executable; frames copied, no offloads (`main.c`, `pal/tcpip/src/main.c`) |
| 42 | Power off works under TCG only; with HVF the `HLT` is a fault (`main.c`, ADR-0008) |
| 43 | `EMUL` is signed, so the clock's seconds fit until 2038 (`timeschdl.mar`, DESIGN-0002) |
| 44 | `vlink` rescans every definition per module taken; `vcdu` limits keywords of a qualifier after a value (`vlink/src/lib.rs`, `vcdu/src/lib.rs`) |
| 45 | `MOVC5` copies forwards only (`vmacro/src/insn.rs`); `apidoc` cuts a line at a `;` inside a string (`apidoc/main.rs`) |

## Testing strategy

- `cargo test -p boot` grows a login: it types a username and password at
  the console, checks the wrong one fails, and runs the rest of its
  transcript as an unprivileged user as well as `SYSTEM`.
- Each privilege check has a line in the transcript: an unprivileged user
  tries the operation and gets `%SYSTEM-F-NOPRIV`, then `SYSTEM` does it.
- File protection: a file one user makes `SET PROTECTION=(W)` is unreadable
  to another in a different group, readable after `(W:R)`.
- `LOCKTEST.EXE`, like `ASTTEST`, checks the lock modes' compatibility,
  conversions, value blocks, blocking ASTs and a deadlock, in two
  processes.
- A batch job submitted in the transcript runs, and its log file is
  `TYPE`d once `SYNCHRONIZE` returns.
- Two logins at once, the console and a `SET HOST` session, open the same
  file for writing, and the second sees `%RMS-E-FLK`. Whether that
  session comes from lwIP's loopback or a second QEMU (PRD-0002's step 6)
  is the first thing step 3 finds out.
- `ods` checks owners and protection on the data disk from the host.

## Open questions

- [ ] Which password hash: something from the host-side standard library
  we can reimplement in MACRO-32 without a big-number library (SHA-256
  with a salt and many rounds), or a real KDF? It must fit `UAF$Q_PWD`'s
  8 bytes, or the record grows past `$UAFDEF`'s.
- [x] Is `SYSUAF.DAT` an indexed file, as on VMS, which RMS doesn't do, or
  a sequential file of fixed records that `AUTHORIZE` rewrites? Indexed,
  as on OpenVMS Alpha V8.4 (VAX/VMS V1.0's was 184-byte fixed records).
  Step 4 waits for PRD-0008's indexed files.
- [ ] Do access control lists come in Milestone 1 after all? Nothing in
  ordinary use needs them; `SYSPRV` and groups cover the cases above.
- [ ] Is the lock manager in the executive, as on VMS, or a process? In the
  executive; it is called at `IPL$_SYNCH` by the file system.
- [ ] Is `PRINT` worth doing without a printer, or only batch queues?

## Work order

Each step ends in something `cargo test -p boot` checks.

1. **Privileges and UICs.** The PCB's privilege masks and UIC, `$SETPRV`,
   `SET PROCESS/PRIVILEGES`, `SHOW PROCESS/PRIVILEGES`. The checks of
   *Privileges, UICs and protection* except files'. *Visible:* `SET
   PROCESS/PRIVILEGES=NOCMKRNL`, then a `$CMKRNL` says
   `%SYSTEM-F-NOPRIV`.
2. **File protection.** Owner and mask from the header, checked by RMS;
   `SET PROTECTION`, `SET FILE/OWNER`, `DIRECTORY/OWNER/PROTECTION`;
   `/OWNER` and `/PROTECTION` on `INITIALIZE` and `MOUNT`. *Visible:* a
   process with another UIC can't read a `(W)` file.
3. **Terminals per UCB.** The console's state moves into its UCB;
   `IO$_SETMODE`/`SENSEMODE` characteristics, `SET TERMINAL`, `SHOW
   TERMINAL`; `TELNETD` makes `TNAn:` units (done, ADR-0027). *Visible:* EDIT's keypad mode
   works in a `SET HOST` session.
4. **UAF and LOGINOUT.** `SYSUAF.DAT`, `AUTHORIZE`, `LOGINOUT`, `SET
   PASSWORD`; `STARTUP` runs `SYSTARTUP_VMS.COM`; the console and `TELNETD`
   start `LOGINOUT`. Needs PRD-0008's steps 1 to 6, indexed files.
   *Visible:* `Username:` at boot; a user made with `AUTHORIZE` logs in
   over `SET HOST` with their UIC and privileges.
5. **Quotas and accounting.** Quotas charged and checked; CPU time and I/O
   counts kept; `$ASCTIM`, `$BINTIM`, `SHOW TIME`. *Visible:* `SHOW
   PROCESS/QUOTAS/ACCOUNTING`; a process that queues reads past `BIOLM`
   waits instead of taking the pool.
6. **Lock manager.** `$ENQ`, `$DEQ`, `$GETLKI`, deadlock detection,
   `LOCKTEST`. *Visible:* `LOCKTEST: ok`.
7. **File system and RMS on locks.** A lock per volume and per file in
   place of `FIL$LOCK`; RMS sharing; `$ERASE` of an open file refused.
   *Visible:* two sessions, `%RMS-E-FLK` on the second open for write.
8. **Command language.** Block `IF`, `ON`, `$STATUS`, `OPEN`/`READ`/`WRITE`/
   `CLOSE`, the lexical functions, image input from the procedure.
   *Visible:* a `LOGIN.COM` that uses `F$MODE()` to skip its terminal
   setup in batch.
9. **Batch.** `JOB_CONTROL`, `$SNDJBC`, `$GETQUI`, the queue commands,
   `SUBMIT`, `SHOW QUEUE`, `SYNCHRONIZE`, `$BRKTHRU` for `/NOTIFY`.
   *Visible:* `SUBMIT` a procedure, `TYPE` its log.
10. **Print, if the open question says so.** A print symbiont to a file or
    terminal. *Visible:* `PRINT WELCOME.TXT` ends in the queue's output.
11. **Tier 2 sweep.** Whatever of tier 2 the steps above didn't touch.
    *Visible:* the backlog's tier 2 is empty.
