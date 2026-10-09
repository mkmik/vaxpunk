# docs

Project-wide documents, one numbered series per kind:

| Directory | Kind | What it records |
| --- | --- | --- |
| `adr/` | Architecture decision record (ADR) | One decision: why it was needed, what was chosen, what was not, and what follows from it |
| `prd/` | Product requirements document (PRD) | What a project or sub-project must deliver, what it won't, and the order of work |
| `design/` | Design document | How something that spans components works: an interface, a protocol, a subsystem |
| `research/` | Research report | How real VMS did something, with sources and what is still unverified: input for an ADR, not a decision |

A format, ABI or on-disk structure that belongs to one component is
documented next to its code, in that component's `docs/` (`crosstools/vtools/docs/`,
`crosstools/ods/docs/`) or README (`pal/shim/README.md`). Those files are named by topic,
not numbered, because the code cites them by path.

[boot.md](boot.md) walks through the boot sequence in plain words. It is
kept current, like a design document.

`api/index.html`, the API reference, is generated from the sources by
`cargo run -p apidoc`: don't edit it, regenerate it (see AGENTS.md).

## Index

### ADRs

| ADR | Title | Status |
| --- | --- | --- |
| [ADR-0001](adr/0001-pal-interface-vms-vocabulary.md) | PAL interface speaks only VMS vocabulary | Accepted |
| [ADR-0002](adr/0002-root-task-is-the-pal.md) | The root task is the PAL; the executive is a task that calls it with privileged instructions | Accepted |
| [ADR-0003](adr/0003-one-cpu-many-threads.md) | Processes are threads that take turns on one VMS CPU | Accepted |
| [ADR-0004](adr/0004-interval-timer-is-a-pal-thread.md) | The interval timer is a periodic thread of the PAL's | Accepted |
| [ADR-0005](adr/0005-access-modes-are-threads.md) | Each access mode of a process is a thread with an address space of its own | Accepted |
| [ADR-0006](adr/0006-cli-in-p1-runs-images-in-its-process.md) | The command interpreter lives in P1 in supervisor mode and runs images in its own process | Accepted |
| [ADR-0007](adr/0007-system-disk-files-11-and-rms.md) | The system disk is a Files-11 volume the PAL reads by LBN, with the file system and RMS in the executive | Accepted |
| [ADR-0008](adr/0008-host-powers-qemu-off-on-halt.md) | On a halt the root task prints a line, and the host powers QEMU off | Accepted |
| [ADR-0009](adr/0009-ramdisk-writable-files-11.md) | MDA0:, a ramdisk the executive drives, holds a Files-11 volume the file system writes | Proposed |
| [ADR-0010](adr/0010-ctrly-calls-the-cli-on-top-of-the-image.md) | CTRL/Y calls the command interpreter on top of the stopped image, and $CONTINUE goes back to it | Superseded by ADR-0014 |
| [ADR-0011](adr/0011-asts-on-the-kernel-stack.md) | The PAL requests AST delivery from bits in the HWPCB, and the executive calls AST routines on top of the kernel stack | Proposed |
| [ADR-0012](adr/0012-data-disk-writable-files-11.md) | DKB0:, a second virtio disk the PAL writes, holds a Files-11 volume that outlives the system | Proposed |
| [ADR-0013](adr/0013-qio-irps-and-drivers.md) | $QIO queues an I/O request packet to the device's driver, and its completion is a kernel mode AST | Proposed |
| [ADR-0014](adr/0014-ctrlc-ctrly-asts.md) | CTRL/C and CTRL/Y are ASTs enabled with IO$_SETMODE, and DCL's CTRL/Y AST stops the image | Proposed |
| [ADR-0015](adr/0015-file-system-io-through-the-disk-driver.md) | The file system hands its block I/O to the disk's start I/O routine in an IRP of its own | Proposed |
| [ADR-0016](adr/0016-tcpip-component-and-bga0.md) | The TCP/IP component is lwIP and a virtio-net driver of our own that the PAL starts, and BGA0: clones a unit per connection | Proposed |
| [ADR-0017](adr/0017-command-tables-from-cld-with-vcdu.md) | Commands are defined in CLD files that vcdu compiles at build time, and a parser library linked into each image reads them | Proposed |
| [ADR-0018](adr/0018-set-command-and-foreign-commands.md) | SET COMMAND compiles CLD in DCL into tables DCL looks in first, and a foreign command's image gets its line as $LINE | Proposed |
| [ADR-0019](adr/0019-mailboxes.md) | A mailbox is a unit of its own, MBnn, whose writes are done at once, and process deletion writes the termination message to it | Proposed |
| [ADR-0020](adr/0020-file-system-lock-below-ipl-synch.md) | The file system runs below IPL$_SYNCH, holding a lock of its own, as the XQP does | Proposed |
| [ADR-0021](adr/0021-condition-handlers-run-in-the-mode-that-signals.md) | Condition handlers are found along the FP chain and run in the mode that signaled, from code in the vector | Proposed |
| [ADR-0022](adr/0022-tcpip-utility-and-dhcp.md) | The network is set with TCP/IP Services' TCPIP utility, which parses its own commands, and lwIP's DHCP client can give the interface its address | Proposed |
| [ADR-0023](adr/0023-calling-standard.md) | The vaxpunk calling standard is AAPCS64 with VMS's argument count, sign extension and self-describing frames, and MACRO-32 is compiled onto it as AMACRO compiled it onto Alpha's | Accepted |
| [ADR-0024](adr/0024-sockets-have-tcpip-services-qio-interface.md) | Sockets have TCP/IP Services' $QIO interface, with UDP and raw ICMP, and the interface is set with its ioctls | Proposed |
| [ADR-0025](adr/0025-hosts-database.md) | Host names come from TCP/IP Services' hosts database, which the programs read themselves, and TCP/IP's files live on the ramdisk, which the startup procedure fills at each boot | Proposed |
| [ADR-0026](adr/0026-name-service-and-nslookup.md) | The resolver's configuration is TCP/IP Services' TCPIP$BIND_* logical names, which SET NAME_SERVICE sets, and the programs ask DNS themselves, through a resolver every image links | Proposed |
| [ADR-0027](adr/0027-terminals-are-ucbs-and-telnet-is-in-the-driver.md) | Each terminal is a UCB of its own, and a remote login is a TELNET terminal, TNAn, that the terminal driver drives over the TCP connection | Proposed |
| [ADR-0028](adr/0028-shareable-images.md) | Shareable images are Alpha's, linked against their symbol vector and called through linker veneers; the image activator maps a copy into each process, and LIBRTL.EXE holds the LIB$ routines | Proposed |

### PRDs

| PRD | Title |
| --- | --- |
| [PRD-0001](prd/0001-vtools.md) | ARM64 cross assembler, linker and QEMU runner for VMS object formats |
| [PRD-0002](prd/0002-networking.md) | TCP/IP through a port to an lwIP component |
| [PRD-0003](prd/0003-multi-user-vms.md) | Milestone 1: a multi-user VMS you log in to |
| [PRD-0004](prd/0004-bliss64-compiler.md) | A BLISS-64 compiler for ARM64, bootstrapped in Rust and then written in BLISS-64 |
| [PRD-0005](prd/0005-macro32-on-the-calling-standard.md) | MACRO-32 on the vaxpunk calling standard, as AMACRO put it on Alpha's |
| [PRD-0006](prd/0006-tpu-and-eve.md) | TPU, the Text Processing Utility, and EVE on it, in BLISS-64 |
| [PRD-0007](prd/0007-wasmrun.md) | WASMRUN, a WebAssembly interpreter for VMS written in MACRO-32 |
| [PRD-0008](prd/0008-rms-record-and-indexed-files.md) | RMS: every record format, relative files and indexed files |

### Design documents

| Design | Title |
| --- | --- |
| [DESIGN-0001](design/0001-pal-interface.md) | PAL interface |
| [DESIGN-0002](design/0002-executive-processes.md) | Processes, memory and system services in the executive |
| [DESIGN-0003](design/0003-tcpip-port.md) | TCP/IP through the port |
| [DESIGN-0004](design/0004-calling-standard.md) | The vaxpunk calling standard |

### Research reports

Named by topic, not numbered, and not kept current: they record what was found when.

| Report | Title |
| --- | --- |
| [openvms-dns](research/openvms-dns.md) | How OpenVMS TCP/IP Services resolves host names (October 2026) |

## Adding a document

1. **Pick the kind.** A choice between alternatives is an ADR. What to
   build, and in which order, is a PRD. How a piece that spans components
   works, in enough detail to implement it, is a design document.
2. **Number it** one above the highest number in its directory, in four
   digits: the first is 0001. Numbers are never reused. If another branch
   merges the same number first, renumber yours before merging.
3. **Name the file** `NNNN-slug.md`, where the slug is a few lowercase words
   naming the subject, joined by hyphens: `0001-pal-interface-vms-vocabulary.md`,
   `0001-vtools.md`.
4. **Start it** with its ID and title, then the date it was written and its
   author. The ID is the directory name in capitals and the number:
   `ADR-0002`, `PRD-0002`, `DESIGN-0001`.

   ```markdown
   # ADR-0002 — Title that states the decision

   Sep 28, 2026 · @Marko Mikulicic
   ```

5. **Add it to the index** above, in the same PR.

Refer to documents by ID, linked: [ADR-0001](adr/0001-pal-interface-vms-vocabulary.md).

### ADRs

- The paragraph after the byline starts with the status, then states the
  decision in a sentence or two. The status is `Proposed`, `Accepted`,
  `Rejected` or `Superseded by ADR-NNNN`.
- Sections as in ADR-0001: `Context`, `Decision`, `Alternatives considered`
  (each option and why not), `Consequences` (what gets harder, what stays
  easy, follow-ups).
- An accepted ADR is not rewritten. To change the decision, write a new ADR
  and set the old one's status to `Superseded by ADR-NNNN`, in the file and
  in the index; that is the only edit it gets. Rejected ADRs stay: they
  record why not.

### PRDs

Sections as in PRD-0001: `Context and goal`, `Non-goals`, the requirements
under whatever headings the subject needs, `Testing strategy`,
`Open questions` and `Work order`, where each step ends with something you
can run or look at.

### Design documents

Open with what is being designed and the ADRs and PRDs it builds on. Unlike
an ADR, a design document is kept current: change it in the same PR as the
code that changes the design.
