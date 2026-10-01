/*
 * The root task: the first user task seL4 starts, holding every capability.
 * It is the PAL (docs/adr/0002-root-task-is-the-pal.md): it shows that seL4
 * handed it a valid boot info page and a scheduling context, loads EXEC.EXE,
 * the MACRO-32 executive, from the boot volume, starts it as a task of its
 * own and serves its PAL calls and faults (docs/design/0001-pal-interface.md)
 * until it halts.
 *
 * The executive's processes are threads in its address space, one seL4 TCB
 * each, and only one of them runs at a time: the PAL hands the one CPU the
 * executive sees from thread to thread when it calls SWPCTX
 * (docs/adr/0003-one-cpu-many-threads.md).
 *
 * The kernel is built with the MCS API (KernelIsMCS in kernel/config.cmake):
 * a thread runs only while it holds a scheduling context with budget left,
 * seL4_Recv and seL4_ReplyRecv take a reply object, and seL4_Reply is gone:
 * seL4_Send on a reply object replies.
 */
#include <stdarg.h>
#include <stdint.h>

#include <sel4/sel4.h>

#ifndef CONFIG_KERNEL_MCS
#error "the root task needs an MCS kernel: set KernelIsMCS in kernel/config.cmake"
#endif

/* Defined here instead of in libsel4, which the root task does not link. */
LIBSEL4_THREAD_LOCAL seL4_IPCBuffer *__sel4_ipc_buffer;

static void putnum(unsigned long v, unsigned base)
{
	char buf[20];
	int n = 0;
	do {
		buf[n++] = "0123456789abcdef"[v % base];
		v /= base;
	} while (v);
	while (n)
		seL4_DebugPutChar(buf[--n]);
}

/* printf subset: %s, %c, %u, %x, %lu, %lx, %%. */
static void __attribute__((format(printf, 1, 2))) print(const char *fmt, ...)
{
	va_list ap;
	va_start(ap, fmt);
	for (; *fmt; fmt++) {
		if (*fmt != '%') {
			seL4_DebugPutChar(*fmt);
			continue;
		}
		int wide = *++fmt == 'l';
		fmt += wide;
		if (*fmt == '%')
			seL4_DebugPutChar('%');
		else if (*fmt == 's')
			for (const char *s = va_arg(ap, const char *); *s; s++)
				seL4_DebugPutChar(*s);
		else if (*fmt == 'c')
			seL4_DebugPutChar(va_arg(ap, int));
		else
			putnum(wide ? va_arg(ap, unsigned long) : va_arg(ap, unsigned),
			       *fmt == 'x' ? 16 : 10);
	}
	va_end(ap);
}

#define PAGE_SIZE (1UL << seL4_PageBits)

static void __attribute__((noreturn)) die(const char *what, seL4_Word v)
{
	print("root task: %s (%lu)\n", what, v);
	seL4_TCB_Suspend(seL4_CapInitThreadTCB);
	for (;;)
		;
}

/* gcc may turn copy and fill loops into these calls, even freestanding. */
void *memcpy(void *dst, const void *src, seL4_Word n)
{
	char *d = dst;
	const char *s = src;
	while (n--)
		*d++ = *s++;
	return dst;
}

void *memset(void *dst, int c, seL4_Word n)
{
	volatile char *d = dst;
	while (n--)
		*d++ = c;
	return dst;
}

static uint32_t u32(const uint8_t *p)
{
	uint32_t v;
	memcpy(&v, p, sizeof v);
	return v;
}

static uint64_t u64(const uint8_t *p)
{
	uint64_t v;
	memcpy(&v, p, sizeof v);
	return v;
}

/* Where retype puts new objects: an untyped cap and the next empty slot. */
static seL4_CPtr untyped, next_slot;

static seL4_CPtr alloc(seL4_Word type, seL4_Word size_bits)
{
	seL4_Error err = seL4_Untyped_Retype(untyped, type, size_bits, seL4_CapInitThreadCNode, 0,
					     0, next_slot, 1);
	if (err)
		die("seL4_Untyped_Retype failed", err);
	return next_slot++;
}

/* A copy of cap, with a badge if it is an endpoint and badge isn't 0. */
static seL4_CPtr mint(seL4_CPtr cap, seL4_Word badge)
{
	seL4_Error err = seL4_CNode_Mint(seL4_CapInitThreadCNode, next_slot, seL4_WordBits,
					 seL4_CapInitThreadCNode, cap, seL4_WordBits,
					 seL4_AllRights, badge);
	if (err)
		die("seL4_CNode_Mint failed", err);
	return next_slot++;
}

/* Maps frame at va in vspace, with page tables as needed. */
static void map(seL4_CPtr frame, seL4_CPtr vspace, seL4_Word va, seL4_CapRights_t rights,
		seL4_ARM_VMAttributes attr)
{
	seL4_Error err;
	while ((err = seL4_ARM_Page_Map(frame, vspace, va, rights, attr)) == seL4_FailedLookup) {
		err = seL4_ARM_PageTable_Map(alloc(seL4_ARM_PageTableObject, 0), vspace, va,
					     seL4_ARM_Default_VMAttributes);
		if (err)
			die("seL4_ARM_PageTable_Map failed", err);
	}
	if (err)
		die("seL4_ARM_Page_Map failed", err);
}

/*
 * Physical memory, as the executive sees it: page frames numbered from 0,
 * PFNs. The executive owns them all and keeps its own PFN database; the PAL
 * makes each frame the first time a PTE names it, and maps it twice: at
 * PHYS + PFN pages in its own address space, the way Alpha PALcode reached
 * memory by physical address, and wherever the executive's PTE says.
 * ponytail: 4 MB, as much as the root CNode's 4096 slots leave room for;
 * frame caps in a CNode of their own for more.
 */
#define PFN_COUNT 1024
#define PHYS 0x100000000UL
static seL4_CPtr pfn_frame[PFN_COUNT]; /* mapped at PHYS */
static seL4_CPtr pfn_cap[PFN_COUNT];   /* mapped in the executive */
static seL4_Word pfn_va[PFN_COUNT];    /* where, 0 if nowhere */

static uint8_t *pfn_ptr(seL4_Word pfn)
{
	if (!pfn_frame[pfn]) {
		pfn_frame[pfn] = alloc(seL4_ARM_SmallPageObject, 0);
		map(pfn_frame[pfn], seL4_CapInitThreadVSpace, PHYS + pfn * PAGE_SIZE,
		    seL4_ReadWrite, seL4_ARM_Default_VMAttributes | seL4_ARM_ExecuteNever);
		pfn_cap[pfn] = mint(pfn_frame[pfn], 0);
	}
	return (uint8_t *)(PHYS + pfn * PAGE_SIZE);
}

/*
 * The executive's address space, one for every process: the system space.
 * Its addresses stay below 2 GB, where MACRO-32's longwords reach. The PAL
 * keeps the PTE it was last given for each page, 4 MB of them to a table.
 * ponytail: 32 tables, 128 MB of scattered address space; more if needed.
 */
#define SPACE_END 0x80000000UL
enum { PTE_VALID = 1u << 31, PTE_EXEC = 1u << 25, PTE_PFN = 0x1fffff, PRT_NA = 0, PRT_KW = 2,
       PRT_KR = 3 };
/* The VAX protection codes kernel mode may write: KW, UW, EW, ERKW, SW... */
#define PRT_KERNEL_WRITES 0x7774
static seL4_CPtr exec_vspace;
static uint32_t *pte_table[SPACE_END >> 22];
static uint32_t pte_pool[32][1024];
static unsigned pte_used;

static uint32_t *pte_at(seL4_Word va, int make)
{
	uint32_t **t = &pte_table[va >> 22];
	if (!*t && make) {
		if (pte_used == sizeof pte_pool / sizeof pte_pool[0])
			die("out of PTE tables", va);
		*t = pte_pool[pte_used++];
	}
	return *t ? &(*t)[va >> seL4_PageBits & 1023] : 0;
}

static int mapped(uint32_t pte)
{
	return pte & PTE_VALID && (pte >> 27 & 15) != PRT_NA;
}

/*
 * WRPTE: makes va's page what pte says, mapped, unmapped or with a new
 * protection, and puts the PTE it had in *old. Fails, returning 0, for an
 * address outside the system space, a reserved protection code, a PFN out
 * of range or already mapped at another address.
 * ponytail: one mapping per PFN; a frame cap per extra mapping for shared
 * pages.
 */
static int wrpte(seL4_Word va, uint32_t pte, uint32_t *old)
{
	unsigned prot = pte >> 27 & 15, pfn = pte & PTE_PFN;
	if (va % PAGE_SIZE || va < PAGE_SIZE || va >= SPACE_END)
		return 0;
	if (mapped(pte) && (prot == 1 || pfn >= PFN_COUNT || (pfn_va[pfn] && pfn_va[pfn] != va)))
		return 0;
	uint32_t *slot = pte_at(va, 1);
	*old = *slot;
	if (mapped(*old)) {
		seL4_Error err = seL4_ARM_Page_Unmap(pfn_cap[*old & PTE_PFN]);
		if (err)
			die("seL4_ARM_Page_Unmap failed", err);
		pfn_va[*old & PTE_PFN] = 0;
	}
	*slot = pte;
	if (!mapped(pte))
		return 1;
	pfn_ptr(pfn);
	seL4_CapRights_t rights = PRT_KERNEL_WRITES >> prot & 1 ? seL4_ReadWrite : seL4_CanRead;
	seL4_ARM_VMAttributes attr = seL4_ARM_Default_VMAttributes;
	if (!(pte & PTE_EXEC))
		attr |= seL4_ARM_ExecuteNever;
	map(pfn_cap[pfn], exec_vspace, va, rights, attr);
	if (pte & PTE_EXEC) {
		seL4_Error err = seL4_ARM_Page_Unify_Instruction(pfn_cap[pfn], 0, PAGE_SIZE);
		if (err)
			die("seL4_ARM_Page_Unify_Instruction failed", err);
	}
	pfn_va[pfn] = va;
	return 1;
}

/* The executive's quadword at va, which must be 8-byte aligned and mapped. */
static uint64_t *quad(seL4_Word va)
{
	uint32_t *pte = va % 8 || va >= SPACE_END ? 0 : pte_at(va, 0);
	if (!pte || !mapped(*pte))
		return 0;
	return (uint64_t *)(PHYS + (*pte & PTE_PFN) * PAGE_SIZE + va % PAGE_SIZE);
}

/* The boot's own PFNs, from 0 up. */
static seL4_Word boot_pfn;

/* Makes pages from va to va + size, filled from src unless it is 0. */
static void boot_pages(seL4_Word va, const uint8_t *src, seL4_Word size, uint32_t pte)
{
	for (seL4_Word off = 0; off < size; off += PAGE_SIZE) {
		if (boot_pfn == PFN_COUNT)
			die("the boot needs more memory than the PAL has", size);
		seL4_Word n = size - off < PAGE_SIZE ? size - off : PAGE_SIZE;
		if (src)
			memcpy(pfn_ptr(boot_pfn), src + off, n);
		uint32_t old;
		if (!wrpte(va + off, PTE_VALID | pte | boot_pfn++, &old))
			die("boot pages: bad address", va + off);
	}
}

/*
 * Maps the sections of an executable image (vtools/docs/image-format.md) at
 * their link addresses, with their protection, and returns the transfer
 * address.
 * ponytail: link address only, no fixups; relocate with the EIAF when an
 * image has to move.
 */
static seL4_Word load_image(const uint8_t *file, seL4_Word size)
{
	enum { BLOCK = 512, EISD_SIZE = 36, GBL = 0x1, DZRO = 0x4, WRT = 0x8, EXE = 0x800 };
	if (size < BLOCK || u32(file) != 3 || u32(file + 52) != 1 || u32(file + 112) != 183)
		die("EXEC.EXE is not an ARM64 executable image", size);
	if (u32(file + 80) & 2)
		die("EXEC.EXE has no transfer address", 0);
	uint32_t hdr = u32(file + 8), at = u32(file + 12), activ = u32(file + 16);
	if (hdr > size || activ > hdr - 16)
		die("EXEC.EXE: bad image header", hdr);
	for (;;) {
		if (at > hdr - 12)
			die("EXEC.EXE: section list runs past the header", at);
		uint32_t eisd = u32(file + at + 8);
		if (eisd == 0)
			break;
		if (eisd == 0xffffffff) {
			at = (at + BLOCK) & ~(BLOCK - 1);
			continue;
		}
		if (eisd < EISD_SIZE || at > hdr - EISD_SIZE)
			die("EXEC.EXE: bad section descriptor", at);
		const uint8_t *d = file + at;
		uint32_t secsize = u32(d + 12), flags = u32(d + 24), vbn = u32(d + 28);
		seL4_Word va = u64(d + 16);
		at += eisd;
		if (flags & GBL || (flags & EXE && flags & WRT) || va % PAGE_SIZE)
			die("EXEC.EXE: bad section flags or address", at);
		const uint8_t *src = 0;
		if (!(flags & DZRO)) {
			if (vbn == 0 || (seL4_Word)(vbn - 1) * BLOCK + secsize > size)
				die("EXEC.EXE: section outside the file", at);
			src = file + (seL4_Word)(vbn - 1) * BLOCK;
		}
		uint32_t pte = (flags & WRT ? PRT_KW : PRT_KR) << 27;
		boot_pages(va, src, secsize, flags & EXE ? pte | PTE_EXEC : pte);
	}
	return u64(file + activ + 8);
}

/*
 * The boot volume (vtools/lib/lib.mlb, $BVDDEF): a directory in its first
 * 512-byte block, entries of a .ASCIC name, a first block and a size, then
 * the files. Returns the file called name, or 0.
 */
static const uint8_t *find_file(const uint8_t *vol, seL4_Word size, const char *name,
				seL4_Word *file_size)
{
	for (const uint8_t *e = vol; e < vol + 512 && *e; e += 32) {
		seL4_Word n = 0;
		while (name[n] && n < *e && e[1 + n] == name[n])
			n++;
		seL4_Word at = (seL4_Word)u32(e + 24) * 512;
		*file_size = u32(e + 28);
		if (n == *e && !name[n] && at <= size && *file_size <= size - at)
			return vol + at;
	}
	return 0;
}

/* The linker's: the root task's first and past-the-last page. */
extern const uint8_t _start[], _end[];

/*
 * What the PAL sets up in the executive's address space before it starts
 * (DESIGN-0001): the restart parameter block, the boot volume, EXEC.EXE's
 * sections and a stack below 2 GB, where MACRO-32's longword addresses reach.
 */
#define RPB_VA 0x1fff0000UL
#define VOLUME_VA 0x20000000UL
#define EXEC_STACK_TOP 0x7fff0000UL
#define EXEC_STACK_PAGES 4
enum { RPB_BASE = 0, RPB_PFNCNT = 4, RPB_FREEPFN = 8, RPB_VOLUME = 12, RPB_VOLSIZE = 16,
       RPB_HWPCB = 64, RPB_LENGTH = 192 };

/*
 * A process context: the thread that runs one executive process, made the
 * first time the executive switches to its hardware PCB. Its seL4 objects
 * outlive it, for the next context in the slot.
 */
#define CTX_MAX 32
static struct ctx {
	seL4_Word hwpcb; /* the executive's HWPCB address; 0: a free slot */
	seL4_CPtr tcb, sc, reply;
	int started;
	/* Its PAL call's message while it waits in one: x0-x7, the svc's PC. */
	seL4_Word mr[seL4_UnknownSyscall_FaultIP + 1];
} ctx[CTX_MAX];
static struct ctx *cur;
static seL4_CPtr fault_ep, sched_control;
static seL4_Time slice;

static struct ctx *find_ctx(seL4_Word hwpcb)
{
	for (struct ctx *c = ctx; c < ctx + CTX_MAX; c++)
		if (c->hwpcb == hwpcb)
			return c;
	return 0;
}

/*
 * A free slot for hwpcb, with a TCB in the executive's address space, an
 * empty CSpace, since it calls nothing but the PAL, and the PAL's endpoint,
 * badged with the slot, for its faults, PAL calls included. It runs below
 * the PAL's priority, so the PAL serves a call as soon as it is made.
 */
static struct ctx *new_ctx(seL4_Word hwpcb)
{
	struct ctx *c = find_ctx(0);
	if (!c)
		return 0;
	if (!c->tcb) {
		c->tcb = alloc(seL4_TCBObject, 0);
		c->sc = alloc(seL4_SchedContextObject, seL4_MinSchedContextBits);
		c->reply = alloc(seL4_ReplyObject, 0);
		seL4_Error err = seL4_TCB_Configure(c->tcb, alloc(seL4_CapTableObject, 1), 0,
						    exec_vspace, 0, 0, seL4_CapNull);
		if (!err)
			err = seL4_SchedControl_Configure(sched_control, c->sc, slice, slice, 0, 0);
		if (!err)
			err = seL4_TCB_SetSchedParams(c->tcb, seL4_CapInitThreadTCB, seL4_MaxPrio - 1,
						      seL4_MaxPrio - 1, c->sc,
						      mint(fault_ep, c - ctx + 1));
		if (err)
			die("configuring a process context failed", err);
	}
	c->hwpcb = hwpcb;
	c->started = 0;
	return c;
}

/* PAL function codes (docs/design/0001-pal-interface.md). */
enum {
	HALT = 0x00, SWPCTX = 0x05, MFPR_IPL = 0x0e, MTPR_IPL = 0x0f, MFPR_PCBB = 0x12,
	MFPR_SCBB = 0x16, MTPR_SCBB = 0x17, MTPR_SIRR = 0x18, MFPR_SISR = 0x19, WTINT = 0x3e,
	MTPR_TXDB = 0x40, WRPTE = 0x41, DELCTX = 0x42, CHMK = 0x83, REI = 0x92
};

/* The system control block's vectors the PAL delivers through. */
enum { SCB_CHMK = 0x40, SCB_SOFTINT = 0x80 };

/*
 * The processor state the PAL keeps for the executive's one CPU. A VAX
 * starts at IPL 31. SISR has a bit per software interrupt level requested
 * and not yet delivered.
 */
static seL4_Word ipl = 31, sisr, scbb;

/* The frame on the kernel stack an interrupt or exception pushes and REI
 * pops ($INTSTKDEF), in quadwords. */
enum { F_PC, F_PS, F_R7, F_SP, F_X_SP, F_X13, F_X30 = F_X13 + 6, F_LENGTH };

/* The PSL's condition codes, NZVC in bits 3:0, and ARM64's NZCV in 31:28. */
static seL4_Word psl(seL4_Word spsr)
{
	return ipl << 16 | (spsr >> 28 & 0xc) | (spsr >> 29 & 1) | (spsr >> 27 & 2);
}

static seL4_Word spsr(seL4_Word psl)
{
	return (psl & 0xc) << 28 | (psl & 1) << 29 | (psl & 2) << 27;
}

/*
 * Delivers an interrupt or exception to the handler the SCB has at off: pushes
 * the frame on the kernel stack, 16-byte aligned, and raises IPL to new_ipl.
 * Returns 0 if there is no SCB, no handler or no stack to push on.
 */
static int vector(seL4_UserContext *r, seL4_Word off, seL4_Word new_ipl)
{
	uint64_t *v = scbb ? quad((scbb + off) & ~7UL) : 0;
	seL4_Word handler = v ? *v >> (scbb + off) % 8 * 8 & 0xffffffff : 0;
	if (!handler)
		return 0;
	seL4_Word frame[F_LENGTH] = { r->pc, psl(r->spsr), r->x7, r->x28, r->sp, r->x13, r->x14,
				      r->x15, r->x16, r->x17, r->x18, r->x30 };
	seL4_Word sp = (r->x28 & ~15UL) - sizeof frame;
	for (unsigned i = 0; i < F_LENGTH; i++) {
		uint64_t *q = quad(sp + 8 * i);
		if (!q)
			return 0;
		*q = frame[i];
	}
	r->x28 = r->sp = sp;
	r->pc = handler;
	ipl = new_ipl;
	return 1;
}

/* Delivers the software interrupts IPL lets through, highest first. */
static int deliver(seL4_UserContext *r)
{
	for (seL4_Word level = 15; level > ipl; level--)
		if (sisr >> level & 1) {
			sisr &= ~(1UL << level);
			if (!vector(r, SCB_SOFTINT + 4 * level, level)) {
				print("%%PAL-F-NOVEC, no handler for software interrupt %lu\n", level);
				return 0;
			}
		}
	return 1;
}

static int deliverable(void)
{
	return sisr >> (ipl + 1) != 0;
}

/* REI: pops the frame at VAX SP. Returns 0 if it isn't one. */
static int rei(seL4_UserContext *r)
{
	seL4_Word f[F_LENGTH];
	for (unsigned i = 0; i < F_LENGTH; i++) {
		uint64_t *q = quad(r->x28 + 8 * i);
		if (!q)
			return 0;
		f[i] = *q;
	}
	/* ponytail: kernel mode only; the other modes come with their tasks. */
	if (f[F_PS] >> 22 & 0xf)
		return 0;
	r->pc = f[F_PC];
	r->spsr = spsr(f[F_PS]);
	r->x7 = f[F_R7];
	r->x28 = f[F_SP];
	r->sp = f[F_X_SP];
	r->x13 = f[F_X13], r->x14 = f[F_X13 + 1], r->x15 = f[F_X13 + 2];
	r->x16 = f[F_X13 + 3], r->x17 = f[F_X13 + 4], r->x18 = f[F_X13 + 5];
	r->x30 = f[F_X30];
	ipl = f[F_PS] >> 16 & 31;
	return 1;
}

static seL4_UserContext regs;

static void read_regs(void)
{
	seL4_Error err = seL4_TCB_ReadRegisters(cur->tcb, 0, 0, sizeof regs / sizeof(seL4_Word),
						&regs);
	if (err)
		die("seL4_TCB_ReadRegisters failed", err);
}

/* Resumes the current context with regs, which it has already been given if
 * it is new. */
static void write_regs(int start)
{
	seL4_Error err = seL4_TCB_WriteRegisters(cur->tcb, start, 0,
						 sizeof regs / sizeof(seL4_Word), &regs);
	if (err)
		die("seL4_TCB_WriteRegisters failed", err);
	if (!start)
		seL4_Send(cur->reply, seL4_MessageInfo_new(0, 0, 0, 0));
}

/*
 * Resumes the current context after its PAL call, with v0 in R0, delivering
 * on the way the interrupts its IPL lets through. A PAL call is an svc with
 * the function code in x7, which seL4 hands over as an unknown syscall
 * fault: the reply sets x0 to x7 and the PC, which must move past the svc,
 * which seL4 would otherwise run again.
 */
static int ret(seL4_Word v0)
{
	seL4_Word *mr = cur->mr;
	if (deliverable()) {
		read_regs();
		regs.pc = mr[seL4_UnknownSyscall_FaultIP] + 4;
		regs.x0 = v0;
		if (!deliver(&regs))
			return 0;
		write_regs(0);
		return 1;
	}
	mr[seL4_UnknownSyscall_X0] = v0;
	mr[seL4_UnknownSyscall_FaultIP] += 4;
	for (unsigned i = 0; i <= seL4_UnknownSyscall_FaultIP; i++)
		seL4_SetMR(i, mr[i]);
	seL4_Send(cur->reply, seL4_MessageInfo_new(0, 0, 0, seL4_UnknownSyscall_FaultIP + 1));
	return 1;
}

/*
 * SWPCTX: the current context waits in its PAL call, holding on to its reply,
 * and the CPU goes to the context of hwpcb. A context that has run returns
 * from its own SWPCTX; a new one starts as REI from the frame at its HWPCB's
 * KSP. Either gets the HWPCB the CPU left in R0, as on Alpha.
 */
static int swpctx(seL4_Word hwpcb)
{
	struct ctx *from = cur, *to = hwpcb ? find_ctx(hwpcb) : 0;
	if (to == from)
		return ret(hwpcb);
	uint64_t *ksp = quad(hwpcb);
	if (!to && ksp)
		to = new_ctx(hwpcb);
	if (!to) {
		print("%%PAL-F-SWPCTX, no context for HWPCB 0x%lx\n", hwpcb);
		return 0;
	}
	cur = to;
	if (to->started) {
		/* Blocked in its own SWPCTX: ret() replies to it. */
		return ret(from->hwpcb);
	}
	to->started = 1;
	regs = (seL4_UserContext){ .x28 = *ksp, .x0 = from->hwpcb };
	if (!rei(&regs)) {
		print("%%PAL-F-SWPCTX, no REI frame at KSP 0x%lx\n", (seL4_Word)*ksp);
		return 0;
	}
	if (!deliver(&regs))
		return 0;
	write_regs(1);
	return 1;
}

/* Delivers CHMK through the SCB, at the same IPL, with the code in R0. */
static int chmk(void)
{
	read_regs();
	regs.pc = cur->mr[seL4_UnknownSyscall_FaultIP] + 4;
	if (!vector(&regs, SCB_CHMK, ipl)) {
		print("%%PAL-F-NOVEC, no CHMK handler or kernel stack\n");
		return 0;
	}
	write_regs(0);
	return 1;
}

static int do_rei(void)
{
	read_regs();
	if (!rei(&regs)) {
		print("%%PAL-F-REI, bad frame at SP 0x%lx, PC 0x%lx\n", (seL4_Word)regs.x28,
		      (seL4_Word)regs.pc);
		return 0;
	}
	if (!deliver(&regs))
		return 0;
	write_regs(0);
	return 1;
}

/*
 * One PAL call or fault from the current context. Returns 0 when the PAL
 * stops serving: the executive halted or took a fault the PAL can't
 * handle yet.
 */
static int serve_one(seL4_MessageInfo_t msg)
{
	switch (seL4_MessageInfo_get_label(msg)) {
	case seL4_Fault_UnknownSyscall:
		break;
	case seL4_Fault_VMFault:
		print("%%PAL-F-ACCVIO, access violation at 0x%lx, PC 0x%lx, HWPCB 0x%lx\n",
		      seL4_GetMR(seL4_VMFault_Addr), seL4_GetMR(seL4_VMFault_IP), cur->hwpcb);
		return 0;
	case seL4_Fault_UserException:
		print("%%PAL-F-OPCDEC, reserved instruction at PC 0x%lx, HWPCB 0x%lx\n",
		      seL4_GetMR(seL4_UserException_FaultIP), cur->hwpcb);
		return 0;
	default:
		print("%%PAL-F-FAULT, fault %lu, HWPCB 0x%lx\n", seL4_MessageInfo_get_label(msg),
		      cur->hwpcb);
		return 0;
	}
	seL4_Word *mr = cur->mr;
	for (unsigned i = 0; i <= seL4_UnknownSyscall_FaultIP; i++)
		mr[i] = seL4_GetMR(i);
	seL4_Word pc = mr[seL4_UnknownSyscall_FaultIP], code = mr[seL4_UnknownSyscall_X7];
	seL4_Word a0 = mr[seL4_UnknownSyscall_X0], a1 = mr[seL4_UnknownSyscall_X1];
	seL4_Word v0 = a0;
	switch (code) {
	case HALT:
		print("%%PAL-I-HALT, halted at PC 0x%lx, R0 %lu\n", pc, a0 & 0xffffffff);
		return 0;
	case SWPCTX:
		return swpctx(a0);
	case MFPR_IPL:
		return ret(ipl);
	case MTPR_IPL:
		v0 = ipl;
		ipl = a0 & 31;
		return ret(v0);
	case MFPR_PCBB:
		return ret(cur->hwpcb);
	case MFPR_SCBB:
		return ret(scbb);
	case MTPR_SCBB:
		scbb = a0 & 0xffffffff;
		return ret(v0);
	case MTPR_SIRR:
		if (a0 & 15)
			sisr |= 1UL << (a0 & 15);
		return ret(v0);
	case MFPR_SISR:
		return ret(sisr);
	case WTINT:
		/* Returns once an interrupt IPL lets through has been delivered.
		 * ponytail: no device interrupts yet, so if none is pending
		 * now, none ever comes. */
		if (!deliverable()) {
			print("%%PAL-I-IDLE, the CPU is idle and no interrupt can come\n");
			return 0;
		}
		return ret(0);
	case MTPR_TXDB:
		seL4_DebugPutChar(a0);
		return ret(v0);
	case WRPTE: {
		uint32_t old;
		if (!wrpte(a0 & 0xffffffff, a1, &old))
			break;
		return ret(old);
	}
	case DELCTX: {
		struct ctx *c = a0 ? find_ctx(a0) : 0;
		if (c == cur)
			break;
		if (c) {
			seL4_TCB_Suspend(c->tcb);
			c->hwpcb = 0;
		}
		return ret(v0);
	}
	case CHMK:
		return chmk();
	case REI:
		return do_rei();
	}
	print("%%PAL-F-OPCDEC, reserved PAL call 0x%lx at PC 0x%lx, R0 0x%lx, R1 0x%lx\n", code,
	      pc, a0, a1);
	return 0;
}

/* Serves the current context's PAL calls and faults; the others wait in a
 * PAL call, SWPCTX, or haven't started. */
static void serve(void)
{
	for (;;) {
		seL4_Word badge;
		seL4_MessageInfo_t msg = seL4_Recv(fault_ep, &badge, cur->reply);
		if (badge != (seL4_Word)(cur - ctx + 1)) {
			print("%%PAL-F-FAULT, a call from context %lu, not the current one\n", badge);
			return;
		}
		if (!serve_one(msg))
			return;
	}
}

/*
 * Starts EXEC.EXE from the boot volume, which seL4 mapped after the root
 * task, in a context of its own whose HWPCB is the RPB's: kernel mode,
 * IPL 31, R11 pointing at the RPB.
 */
static void start_exec(seL4_BootInfo *bi)
{
	exec_vspace = alloc(seL4_ARM_VSpaceObject, 0);
	seL4_Error err = seL4_ARM_ASIDPool_Assign(seL4_CapInitThreadASIDPool, exec_vspace);
	if (err)
		die("seL4_ARM_ASIDPool_Assign failed", err);
	/* seL4 maps the user image, the boot volume included, frame after frame from _start. */
	seL4_Word mapped_size = (bi->userImageFrames.end - bi->userImageFrames.start) << seL4_PageBits;
	seL4_Word vol_size = (seL4_Word)_start + mapped_size - (seL4_Word)_end, exe_size;
	const uint8_t *exe = find_file(_end, vol_size, "EXEC.EXE", &exe_size);
	if (!exe)
		die("no EXEC.EXE on the boot volume", vol_size);
	seL4_Word entry = load_image(exe, exe_size);
	boot_pages(VOLUME_VA, _end, vol_size, PRT_KR << 27);
	boot_pages(EXEC_STACK_TOP - EXEC_STACK_PAGES * PAGE_SIZE, 0, EXEC_STACK_PAGES * PAGE_SIZE,
		   PRT_KW << 27);
	boot_pages(RPB_VA, 0, PAGE_SIZE, PRT_KW << 27);
	uint8_t *rpb = (uint8_t *)quad(RPB_VA);
	uint32_t fields[] = { [RPB_BASE / 4] = RPB_VA,	      [RPB_PFNCNT / 4] = PFN_COUNT,
			      [RPB_FREEPFN / 4] = boot_pfn,   [RPB_VOLUME / 4] = VOLUME_VA,
			      [RPB_VOLSIZE / 4] = vol_size };
	memcpy(rpb, fields, sizeof fields);

	cur = new_ctx(RPB_VA + RPB_HWPCB);
	cur->started = 1;
	regs = (seL4_UserContext){ .pc = entry, .sp = EXEC_STACK_TOP, .x28 = EXEC_STACK_TOP,
				   .x11 = RPB_VA };
	write_regs(1);
	print("EXEC.EXE: started at 0x%lx, %lu of %u pages in use\n", entry, boot_pfn, PFN_COUNT);
}

int main(seL4_BootInfo *bi)
{
	seL4_SetIPCBuffer(bi->ipcBuffer);
	print("hello from the root task\n");

	seL4_Word n = bi->untyped.end - bi->untyped.start;
	print("boot info: node %lu of %lu, %lu untyped caps\n", bi->nodeID, bi->numNodes, n);
	for (seL4_Word i = 0; i < n; i++) {
		seL4_UntypedDesc *u = &bi->untypedList[i];
		print("  untyped %lu: paddr 0x%lx size 2^%u%s\n", i, u->paddr, u->sizeBits,
		      u->isDevice ? " device" : "");
	}

	/*
	 * seL4 starts the root task on seL4_CapInitThreadSC with a round-robin
	 * budget of CONFIG_BOOT_THREAD_TIME_SLICE ms per period of the same
	 * length. Reconfigure it to the same values through the node's sched
	 * control cap to show the MCS calls work; times are in microseconds.
	 */
	sched_control = bi->schedcontrol.start;
	print("sched control caps: %lu-%lu\n", sched_control, bi->schedcontrol.end - 1);
	slice = CONFIG_BOOT_THREAD_TIME_SLICE * 1000;
	seL4_Error err = seL4_SchedControl_Configure(sched_control, seL4_CapInitThreadSC, slice,
						       slice, 0, 0);
	if (err)
		print("seL4_SchedControl_Configure failed: %u\n", err);
	seL4_SchedContext_Consumed_t used = seL4_SchedContext_Consumed(seL4_CapInitThreadSC);
	if (used.error)
		print("seL4_SchedContext_Consumed failed: %u\n", used.error);
	else
		print("scheduling context: budget %lu us per %lu us, %lu us used\n", slice, slice,
		      used.consumed);

	/* The largest RAM untyped; empty slots follow the boot info's caps. */
	seL4_Word best = n;
	for (seL4_Word i = 0; i < n; i++)
		if (!bi->untypedList[i].isDevice &&
		    (best == n || bi->untypedList[i].sizeBits > bi->untypedList[best].sizeBits))
			best = i;
	if (best == n)
		die("no RAM untyped", n);
	untyped = bi->untyped.start + best;
	next_slot = bi->empty.start;
	fault_ep = alloc(seL4_EndpointObject, 0);
	start_exec(bi);
	serve();

	print("root task done, %lu of %u slots used\n", next_slot, 1u << CONFIG_ROOT_CNODE_SIZE_BITS);
	err = seL4_TCB_Suspend(seL4_CapInitThreadTCB);
	print("seL4_TCB_Suspend failed: %u\n", err);
	return 0;
}
