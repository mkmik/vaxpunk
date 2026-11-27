# ADR-0050 — A fault on a page whose PTE isn't valid but allows the access is translation not valid, which the executive takes in any mode, and $CRETVA and $EXPREG make demand zero pages

Nov 27, 2026 · @Marko Mikulicic

Proposed. A PTE whose valid bit is clear but whose enables aren't 0 is
a page the executive has promised but not made yet. The PAL keeps it as
`WRPTE` wrote it. An access its enables allow is a translation not valid
fault, which the PAL delivers through the SCB's vector at `0x24`, the
VAX's, in kernel mode, from any mode, kernel mode too. The executive's
handler, `MMG$PAGEFAULT`, makes the page and returns to the instruction,
which runs again. `$CRETVA` and `$EXPREG` make such pages, demand zero
pages, as VMS's do: a page takes a PFN only when it is first touched,
though one is kept free for it from the start.
`t8y:/exec/memory#page-faults` and `t8y:/platform/pal#page-faults`.

## Context

Since [ADR-0037](0037-all-ram-and-address-spaces-filled-on-faults.md) the
PAL fills each mode's address space on faults: a fault on a page whose
PTE is valid and lets the mode make the access maps the page and goes
on, unseen. Any other fault was an access violation, and in kernel mode
it halted the system. So every page had to exist before anything touched
it: `$CRETVA` and `$EXPREG` took a PFN for each page and zeroed it at
once, and a program that asked for a large region waited for all of it
to be mapped and zeroed
before using any. Paging, an image's sections read as they are touched,
and global sections of a file (`t8y:/exec/paging`,
`t8y:/exec/image-activator#demand-paging`) all need the executive to
hear about a page that isn't there and fix it. ADR-0037 left room for
that: "a fault on a page whose PTE isn't valid can go to the executive
as a translation-not-valid fault".

On the VAX, the memory management unit checks a page's protection
before its valid bit: an access the protection code refuses is an access
violation, whether the page is there or not; one it allows, on a page
whose PTE isn't valid, is translation not valid, vector `0x24`, with the
same parameters, a mask and the address. VMS's handler, `MMG$PAGEFAULT`,
finds what the invalid PTE says the page is, a demand zero page, one in
the page file or in an image file, makes it valid, and goes back to the
instruction. Kernel mode takes page faults too, up to `IPL$_APCDEL`: a
system service writes its caller's buffers as the caller would. Above
that, a fault is the `PGFIPLHI` bugcheck.

## Decision

1. **The PTE.** A PTE with the valid bit clear and some enables set is
   kept by the PAL as `WRPTE` gave it, in the same tables as the valid
   ones; `RDPTE` returns it. Its enables are a protection code's, like
   a valid PTE's, or `WRPTE` refuses it. A PTE of 0 is no page. The
   other bits, PFN included, are the executive's; a demand zero page
   has PFN 0 and bit 16, `DZRO` in `memory.mar`, which marks it as one
   whatever its protection, no access included. Its table of PTEs comes from the PAL's
   share, so a `WRPTE` of one that needs a table the share has no room
   for returns -1, as a mapping's does, and changes nothing.
2. **The fault.** A fault whose PTE isn't valid but whose enables let
   the mode make the access, read, write or execute, is translation not
   valid: the PAL delivers it through `SCB$L_TNV`, `0x24`, with the
   access violation's parameters, a mask with bit 2 set for a write, and
   the address, and the PC of the instruction that faulted. It does so
   in kernel mode, at the same IPL, as every exception; unlike the
   others, from kernel mode too. A queue call whose link is such a page
   is one too, with the call's PC, so the call runs again. Any other
   fault stays an access violation, checked first, as on the VAX.
3. **PROBE** checks the enables whether the PTE is valid or not, as the
   VAX's `PROBE` checks the protection code: a demand zero page the
   mode may write passes `PROBEW`, so a service may write it for its
   caller, and its own fault then makes it.
4. **The handler.** `MMG$PAGEFAULT` makes the page, zeroed, with the
   PTE's protection and the valid bit, and REIs. It makes the demand
   zero pages that follow it too, up to 16 in all, 64 KB, as VMS reads a
   cluster of pages on a fault: a program that touches a region in order
   takes a sixteenth of the faults. A fault costs a dozen trips through
   the PAL and two thread switches: with a page a fault, `MEMTEST`, which
   touches 1.5 GB, took nearly five times as long as with the pages made
   by `$EXPREG`.
   It may take a fault at `IPL$_SYNCH` or below; above it, it halts,
   VMS's `PGFIPLHI`, since the PFN database may be half changed. VMS's
   limit is `IPL$_APCDEL`, because a fault there may wait for a read;
   a demand zero fault never waits, and vaxpunk's executive touches its
   callers' pages at `IPL$_SYNCH`, as the console's writes do. If the
   PAL has no room to map the page, its share used up by page tables
   and threads after it dropped every mapping it could, an outer mode's
   fault is an access violation, and kernel mode's halts.
   ponytail: a halt, where a `$CRETVA` used to fail; a PAL that gives
   back page tables, or keeps room for each demand zero page, if it
   shows.
5. **Demand zero pages.** `$CRETVA` and `$EXPREG` write PTEs with the
   user-write enables, fault on execute and no valid bit. Each keeps a
   free PFN for itself, counted in `MMG$GL_DZROCNT`, as VMS charges a
   demand zero page to the page file: `MMG$ALLOCPFN` hands out only the
   PFNs past those, and `$CRETVA` and `$EXPREG` fail with
   `SS$_INSFMEM` when none is left to keep, as they did when they made
   the pages. So a fault always finds a PFN, and no process can make
   another's fault in kernel mode run out. The rest of the executive's
   own pages, pool, stacks and image sections, are made at once with
   `MMG$CREPAG`, as before.
6. **What reaches a page without the CPU.** `MMG$IOLOCK` makes a demand
   zero page before it locks it, as VMS's faults it in, since the device
   needs a PFN. `$GET_ENTROPY` touches its buffer's first and last
   bytes, at most two pages, before the PAL writes it.

## Alternatives considered

- **The PAL zeroes a page itself on the fault.** It has every PFN at an
  address of its own, but not the PFN database: it would have to ask the
  executive for a free PFN, or own the free list, which ADR-0037 gives
  the executive. And paging needs the executive anyway.
- **A vaxpunk exception, or a PAL call the handler answers.** The VAX's
  vector, its parameters and REI to the faulting instruction are
  everything the executive needs, and what VMS's code expects.
- **Faults only at `IPL$_APCDEL` and below, as VMS.** The terminal
  driver's writes and services that hold `IPL$_SYNCH` while they write a
  caller's buffer would halt the system on a demand zero page. Lowering
  the limit belongs to paging, when a fault can wait.
- **No PFNs kept: a fault out of PFNs waits, or fails.** VMS puts a
  process out of PFNs in the `FPG` state until pages are freed; without
  paging nothing frees pages for it, so it would wait for other
  processes to exit. Failing it is an access violation in an outer mode
  and a halt in kernel mode, where a service writes its caller's page:
  any process that took every PFN could stop the system.

## Consequences

**What gets easier.**
- A page takes a frame of memory, and is zeroed, only when it is
  touched; memory is still counted as taken when it is asked for.
- Paging (`t8y:/exec/paging#faults`) and image sections read page by
  page (`t8y:/exec/image-activator#demand-paging`) are more kinds of
  invalid PTE for `MMG$PAGEFAULT`.

**What gets harder.**
- A region's pages are no longer known to exist: code that reads PTEs
  (`RDPTE`) must expect invalid ones with enables, as `MMG$IOLOCK` and
  `MMG$SETPRT` do.
- Kernel mode can take a fault anywhere it touches a caller's pages, so
  it must do so at `IPL$_SYNCH` or below, and not between steps of a
  change to the PFN database.
- A PFN kept for a demand zero page is a PFN pool and image sections
  can't have; `SYI$_FREE_PAGES` counts them as free.
- A fault makes up to 15 pages more than it needs, which a program that
  touches pages here and there pays for.
- A PAL call that reads or writes memory by address, `GETENTROPY` and
  the boot's `READLBLK` and `WRITELBLK`, can't fault: its caller makes
  the pages first.

**Follow-ups:** DESIGN-0001's *Memory* and *Exceptions*; DESIGN-0002's
*Memory*; PRD-0003 tier 3 #21.
