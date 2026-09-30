/*
 * The root task: the first and only user task seL4 starts, holding every
 * capability. For now it shows that seL4 handed it a valid boot info page and
 * a scheduling context, loads exec.exe, the MACRO-32 part, calls it and
 * prints what it returns, then suspends itself.
 *
 * The kernel is built with the MCS API (KernelIsMCS in kernel/config.cmake):
 * a thread runs only while it holds a scheduling context with budget left,
 * seL4_Recv and seL4_ReplyRecv take a reply object, and seL4_Reply is gone.
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

/* printf subset: %s, %c, %u, %x, %lu, %lx. */
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
		if (*fmt == 's')
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

/* gcc may turn copy loops into memcpy calls, even freestanding. */
void *memcpy(void *dst, const void *src, seL4_Word n)
{
	char *d = dst;
	const char *s = src;
	while (n--)
		*d++ = *s++;
	return dst;
}

/* Where retype puts new objects: an untyped cap and the next empty slot. */
static seL4_CPtr untyped, next_slot;

static seL4_CPtr alloc(seL4_Word type)
{
	seL4_Error err = seL4_Untyped_Retype(untyped, type, 0, seL4_CapInitThreadCNode, 0, 0,
					     next_slot, 1);
	if (err)
		die("seL4_Untyped_Retype failed", err);
	return next_slot++;
}

/* Maps a new, zeroed page at va, read-write, with page tables as needed. */
static seL4_CPtr map_page(seL4_Word va)
{
	seL4_CPtr frame = alloc(seL4_ARM_SmallPageObject);
	seL4_Error err;
	while ((err = seL4_ARM_Page_Map(frame, seL4_CapInitThreadVSpace, va, seL4_ReadWrite,
					seL4_ARM_Default_VMAttributes)) == seL4_FailedLookup) {
		err = seL4_ARM_PageTable_Map(alloc(seL4_ARM_PageTableObject),
					     seL4_CapInitThreadVSpace, va,
					     seL4_ARM_Default_VMAttributes);
		if (err)
			die("seL4_ARM_PageTable_Map failed", err);
	}
	if (err)
		die("seL4_ARM_Page_Map failed", err);
	return frame;
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

/*
 * Maps the sections of an executable image (vtools/docs/image-format.md) at
 * their link addresses in the root task's own address space, with their
 * protection, and returns the transfer address.
 * ponytail: link address only, no fixups; relocate with the EIAF when an
 * image has to move.
 */
static seL4_Word load_image(const uint8_t *file, seL4_Word size)
{
	enum { BLOCK = 512, EISD_SIZE = 36, GBL = 0x1, DZRO = 0x4, WRT = 0x8, EXE = 0x800 };
	if (size < BLOCK || u32(file) != 3 || u32(file + 52) != 1 || u32(file + 112) != 183)
		die("exec.exe is not an ARM64 executable image", size);
	if (u32(file + 80) & 2)
		die("exec.exe has no transfer address", 0);
	uint32_t hdr = u32(file + 8), at = u32(file + 12), activ = u32(file + 16);
	if (hdr > size || activ > hdr - 16)
		die("exec.exe: bad image header", hdr);
	for (;;) {
		if (at > hdr - 12)
			die("exec.exe: section list runs past the header", at);
		uint32_t eisd = u32(file + at + 8);
		if (eisd == 0)
			break;
		if (eisd == 0xffffffff) {
			at = (at + BLOCK) & ~(BLOCK - 1);
			continue;
		}
		if (eisd < EISD_SIZE || at > hdr - EISD_SIZE)
			die("exec.exe: bad section descriptor", at);
		const uint8_t *d = file + at;
		uint32_t secsize = u32(d + 12), flags = u32(d + 24), vbn = u32(d + 28);
		seL4_Word va = u64(d + 16);
		at += eisd;
		if (flags & GBL || (flags & EXE && flags & WRT) || va % PAGE_SIZE)
			die("exec.exe: bad section flags or address", at);
		const uint8_t *src = 0;
		if (!(flags & DZRO)) {
			if (vbn == 0 || (seL4_Word)(vbn - 1) * BLOCK + secsize > size)
				die("exec.exe: section outside the file", at);
			src = file + (seL4_Word)(vbn - 1) * BLOCK;
		}
		seL4_CapRights_t rights = flags & WRT ? seL4_ReadWrite : seL4_CanRead;
		seL4_ARM_VMAttributes attr = seL4_ARM_Default_VMAttributes;
		if (!(flags & EXE))
			attr |= seL4_ARM_ExecuteNever;
		for (seL4_Word off = 0; off < secsize; off += PAGE_SIZE) {
			seL4_Word n = secsize - off < PAGE_SIZE ? secsize - off : PAGE_SIZE;
			seL4_CPtr frame = map_page(va + off);
			if (src)
				memcpy((void *)(va + off), src + off, n);
			/* Mapping it again at the same address changes its rights. */
			seL4_Error err = seL4_ARM_Page_Map(frame, seL4_CapInitThreadVSpace, va + off,
							   rights, attr);
			if (!err && flags & EXE)
				err = seL4_ARM_Page_Unify_Instruction(frame, 0, n);
			if (err)
				die("seL4_ARM_Page_Map failed", err);
		}
	}
	return u64(file + activ + 8);
}

/* The linker's: the root task's first and past-the-last page. */
extern const uint8_t _start[], _end[];
unsigned exe_call(seL4_Word entry);

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
	seL4_CPtr sched_control = bi->schedcontrol.start;
	print("sched control caps: %lu-%lu\n", sched_control, bi->schedcontrol.end - 1);
	seL4_Time slice = CONFIG_BOOT_THREAD_TIME_SLICE * 1000;
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
	/* seL4 maps the user image, exec.exe included, frame after frame from _start. */
	seL4_Word mapped = (bi->userImageFrames.end - bi->userImageFrames.start) << seL4_PageBits;
	seL4_Word entry = load_image(_end, (seL4_Word)_start + mapped - (seL4_Word)_end);
	print("exec.exe: calling 0x%lx\n", entry);
	print("exec.exe: returned %u\n", exe_call(entry));

	print("root task done\n");
	err = seL4_TCB_Suspend(seL4_CapInitThreadTCB);
	print("seL4_TCB_Suspend failed: %u\n", err);
	return 0;
}
