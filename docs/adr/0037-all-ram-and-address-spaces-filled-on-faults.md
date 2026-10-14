# ADR-0037 — Every page of RAM is a PFN, address spaces are filled on faults as a TLB is, and a PTE is Alpha's quadword

Oct 14, 2026 · @Marko Mikulicic

Accepted. The PAL takes all the RAM seL4 gives it, keeps a share for its
own objects and gives the executive the rest as PFNs, with its
capabilities in CNodes it makes as needed. The seL4 address spaces of a
process's threads are a cache of the PTEs the executive writes, filled
when a thread faults, as Alpha PALcode filled the TLB, so kernel mode has
an address space per process and a process switch maps nothing. A PTE is
Alpha's quadword, with a 32-bit PFN. The executive sizes its PFN
database, pool and process table at boot. This replaces decision 3 of
[ADR-0005](0005-access-modes-are-threads.md); its other decisions stand.
It is step 11 of [PRD-0003](../prd/0003-multi-user-vms.md)'s work order.

## Context

vaxpunk is meant to run real programs, and PRD-0003 says no component
should be held back by a size chosen for the demo. Memory is held back at
every layer between QEMU and a program:

- **RAM.** seL4 builds QEMU's memory map in when it is configured
  (`QEMU_MEMORY`, from `QEMU_MEM` in `pal/kernel/qemu.env`: 1,024 MB) and
  gives the root task what the kernel doesn't use as untyped
  capabilities. It can't be told another size at boot. QEMU gives the
  guest's RAM host pages only as they are first touched, so a large `-m`
  costs the host nothing until it is used.
- **Frames.** The PAL makes a PFN's frame the first time a PTE names it,
  from the largest RAM untyped, and stops at 1,024 PFNs, 4 MB. Each frame,
  each other mapping of it in each mode's address space, each seL4 page
  table, thread and address space takes one of the root CNode's 4,096
  slots. The PAL's own tables are static arrays: 96 PTE tables, 256 more
  mappings of a PFN, 32 process contexts, a 512 KB buffer for `EXEC.EXE`.
- **Process switches.** seL4 clears a page table when it is unmapped, so
  kernel mode's address space, which is the executive's, holds the current
  process's P0 and P1 (ADR-0005), and `SWPCTX` unmaps the old process's
  pages there and maps the new one's, a seL4 call a page. That was cheap
  for programs of a few pages; two programs of 1 GB would cost some
  500,000 calls a switch. It also assumes one CPU: two CPUs in kernel mode
  in two processes can't share one address space.
- **The PTE.** DESIGN-0001's PTE is the VAX's longword, with a 21-bit
  PFN: 8 GB of 4 KB pages, however many PFNs the PAL has.
- **The executive.** Its free PFN list, `MMG$AL_FREEPFN`, is 4,096
  longwords in `EXEC.EXE`; pool is 512 KB; `SCH$C_MAXPIX` fixes the
  process table.

One size stays. The 32-bit space is 2 GB: P0 1 GB, S0 and P1 512 MB each
(ADR-0005), since a sign-extended longword at or above `0x80000000` is an
address in seL4's half. One program reaches about 1.5 GB through 32-bit
addresses, and more through P2 (`t8y:/exec/64-bit-addressing`), as on
OpenVMS Alpha. Many processes together reach all of RAM.

On Alpha the TLB was a cache of the page tables the executive wrote: a
miss trapped to PALcode, which read the PTE and filled the entry, and the
executive invalidated an entry with `MTPR #PR$_TBIS` after changing its
PTE. Address space numbers tagged the entries, so a process switch
didn't flush them. Alpha's PTE is a quadword: the valid bit in bit 0, an
enable for each mode's reads and writes in bits 8 to 15, and the PFN in
bits 32 to 63.

## Decision

1. **RAM is one setting.** `QEMU_MEM` builds seL4's memory map and gives
   QEMU its `-m`, as now, and becomes 8,192 MB, in CI too. Another size is
   a rebuild of the kernel, which cargo does when `qemu.env` changes.
   QEMU must give at least that much: seL4 puts the root task's objects
   at the top of the RAM it was built for, so with less it doesn't boot,
   and with more it never uses the rest.
2. **The PAL takes every RAM untyped and keeps a share.** At boot it sets
   aside 1/32 of RAM, and at least 16 MB, for its own objects: CNodes,
   seL4 page tables, threads, address spaces, its PTE tables and its
   records of mappings. The rest is PFNs, `RPB$L_PFNCNT` of them, each
   made the first time a PTE names it, as now. Each PFN costs the share a
   little over 1%: a capability slot, the PAL's own mapping of the frame,
   and the head of its list of mappings. The share is a constant in
   `main.c`, to be tuned when measured.
3. **Capabilities live in CNodes made as needed.** The PAL moves its
   CSpace to two levels: a root CNode sized from RAM, whose slots hold
   CNodes of 4,096 slots, each made from the share when the last fills. A
   PFN's frame capability is at a slot computed from the PFN. Freed slots
   are used again.
4. **Address spaces are filled on faults.** The PTEs the PAL keeps from
   `WRPTE` (decision 6) are the only page tables. Each thread's seL4
   address space caches them, as a TLB does: when a thread faults on a
   page whose PTE lets its mode make that access, the PAL maps the page
   and resumes the thread. Any other fault is an access violation, as now,
   and stops the system in kernel mode. `WRPTE` unmaps the page from every
   address space that has it, as `TBIS` invalidated a TLB entry. It may
   also map the page at once where it is about to be used: in the current
   process's kernel mode, which zeroes new pages. The PAL may drop any
   cached mapping when its share runs low, and fill it on the next fault.
5. **Kernel mode has an address space per process.** Each process's
   kernel-mode thread runs in an address space of its own, holding its P0
   and P1 and the S0 pages it has touched, as each outer mode's does.
   `SWPCTX` maps and unmaps nothing. An address space is the PAL's address
   space number: seL4 has 65,536 ASIDs, in pools of 512 that the PAL makes
   as needed, enough for some 16,000 processes using all four modes. What each
   mode may reach is unchanged (ADR-0005 decision 4, DESIGN-0001's
   protection rules).
6. **A PTE is Alpha's quadword.** `WRPTE` takes and returns 64 bits laid
   out as on Alpha: valid in bit 0, fault on execute in bit 3, the read
   enables of kernel, executive, supervisor and user mode in bits 8 to 11
   and their write enables in bits 12 to 15, the PFN in bits 32 to 63. FOR,
   FOW, ASM and the granularity hint are 0. `$SETPRT`'s and `$CRETVA`'s
   callers still give `$PRTDEF` codes, which the executive turns into
   enables with a table. The PAL keeps PTEs in 4 KB tables of 512, each
   mapping 2 MB, made from its share as needed under a directory for each
   process and one for S0. P2 and S2 add levels above them, not a format.
   A `WRPTE` the share has no room for fails, and the executive returns
   `SS$_INSFMEM`.
7. **The executive sizes itself at boot.** S0 keeps ADR-0005's 512 MB,
   laid out at boot: `EXEC.EXE`, then the PFN database, then pool; the
   port, the RPB, the ramdisk's blocks and the boot stack keep their
   places. The PFN database is sized from `RPB$L_PFNCNT`, 8
   bytes a PFN today, and boot doesn't walk it: PFNs past a high-water
   mark are free without being listed. It takes at most 128 MB of S0,
   which indexes 64 GB of RAM; past that the executive uses the PFNs it
   can index and says so on the console, until the database moves to S2.
   Pool starts at `NPAGEDYN` and grows a page at a time up to `NPAGEVIR`,
   the end of its region. The process table holds `MAXPROCESSCNT`
   processes. The PAL's buffer for `EXEC.EXE` is sized from the file.
8. **A test fills the space.** `MEMTEST` runs in the boot test: one
   process `$EXPREG`s P0 to its end and touches every page, and several
   together take more than 4 GB. *Visible:* `MEMTEST: ok`.

## Alternatives considered

| Option | Why not |
| --- | --- |
| A larger root CNode, `KernelRootCNodeSizeBits` sized from `QEMU_MEM` | At most 2^26 slots, all made at boot whether used or not, and still a fixed number of mappings and contexts. |
| Keep kernel mode's one address space and remap on `SWPCTX` (ADR-0005) | A switch costs a seL4 call a page of both processes, and only one CPU can be in kernel mode. |
| One kernel address space, filled on faults and emptied on `SWPCTX` | A switch costs a call for each page the old process touched in kernel mode, and two CPUs still can't share it. |
| A kernel address space per process with all of S0 mapped in each at once | A capability per S0 page per process, and a call per process for each S0 `WRPTE`, for pages most processes never touch. |
| Page tables in the executive's memory, which the PAL reads on a fault as Alpha PALcode did | ADR-0001 chose explicit calls; the PAL would still need `WRPTE` to invalidate, and would trust for `PROBER` and its own reads tables any stray kernel write can change. |
| The VAX longword with the PFN in bits 32 to 63 | MACRO-32 sign-extends a longword result and the VAX's valid bit is bit 31, so each `BISL` that builds a PTE would fill the PFN with ones. |
| The VAX longword with more PFN bits from its unused ones | 25 bits, 128 GB: another ceiling. |
| The PFN database in S2 now | It needs S2 in the PAL and 64-bit addresses in the executive's MACRO-32 first; S0 holds 64 GB's worth until then. |
| RAM size from the DTB at boot | seL4 builds the memory map in; it would be a kernel change of our own. |

## Consequences

**What gets harder.**
- A page's first touch in each address space is a fault the PAL serves:
  a process's first run faults in the S0 pages it uses. ponytail: fill a
  new process's kernel mode with the S0 pages every process touches if
  that shows in a profile.
- The PAL's objects come and go: a CSpace of two levels, ASID pools, page
  tables and capabilities used again when a cached mapping is dropped.
- Every place that builds or reads a PTE changes: `$PTEDEF`,
  `memory.mar`, `process.mar` and `main.c`.
- Another RAM size is a kernel rebuild.
- The PFN database in S0 caps RAM at 64 GB until it moves to S2.

**What stays easy.**
- The executive's view: `WRPTE`, `PROBER`, the regions and the
  protection codes.
- A process switch costs the same whatever the processes' sizes, and a
  CPU per process in kernel mode is possible (`t8y:/platform/smp`).
- An S0 page outer modes may read can change protection after they run,
  since `WRPTE` unmaps it wherever it is cached.
- Paging builds on the fault path: a fault on a page whose PTE isn't
  valid can go to the executive as a translation-not-valid fault
  (`t8y:/platform/pal#page-faults`) instead of an access violation.
- P2 and S2 are more of the same: levels above the PTE tables, and more
  addresses `WRPTE` takes.

**Follow-ups:** DESIGN-0001's *Memory* and *Process contexts* and
DESIGN-0002 change with the code; P2, S2 and the PFN database in S2
(`t8y:/exec/64-bit-addressing`); paging (`t8y:/exec/paging`); `$EXPREG` of
P1 (`t8y:/exec/memory#p1-expreg`).
