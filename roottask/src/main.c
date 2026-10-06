/*
 * The root task: the first user task seL4 starts, holding every capability.
 * It is the PAL (docs/adr/0002-root-task-is-the-pal.md): it shows that seL4
 * handed it a valid boot info page and a scheduling context, drives the
 * system disk, loads EXEC.EXE, the MACRO-32 executive, from it, starts it
 * as a task of its own and serves its PAL calls and faults
 * (docs/design/0001-pal-interface.md) until it halts.
 *
 * The executive's processes are threads in its address space, one seL4 TCB
 * each, and only one of them runs at a time: the PAL hands the one CPU the
 * executive sees from thread to thread when it calls SWPCTX
 * (docs/adr/0003-one-cpu-many-threads.md). A clock thread of the PAL's
 * ticks every 10 ms, and the PAL delivers each tick to the executive as the
 * VAX's interval timer interrupt, preempting the current thread
 * (docs/adr/0004-interval-timer-is-a-pal-thread.md).
 *
 * A process also has a thread for each outer access mode it enters,
 * executive, supervisor and user, each in an address space of its own that
 * holds the pages that mode may read; CHMx, REI, interrupts and exceptions
 * move the CPU between a process's threads
 * (docs/adr/0005-access-modes-are-threads.md). The kernel-mode threads share
 * the executive's address space, whose P0 and P1 are the current process's.
 *
 * The kernel is built with the MCS API (KernelIsMCS in kernel/config.cmake):
 * a thread runs only while it holds a scheduling context with budget left,
 * seL4_Recv and seL4_ReplyRecv take a reply object, and seL4_Reply is gone:
 * seL4_Send on a reply object replies.
 */
#include <stdarg.h>
#include <stdint.h>

#include <sel4/sel4.h>

#include "component.h"
#include "port.h"

#ifndef CONFIG_KERNEL_MCS
#error "the root task needs an MCS kernel: set KernelIsMCS in kernel/config.cmake"
#endif

/* Defined here instead of in libsel4, which the root task does not link. */
LIBSEL4_THREAD_LOCAL seL4_IPCBuffer *__sel4_ipc_buffer;

/*
 * The console: QEMU virt's PL011 UART, which the PAL drives itself, so it
 * needs no debug kernel. A debug kernel prints on it too.
 * ponytail: QEMU virt's address; read it from the DTB for other boards.
 */
#define UART_PADDR 0x09000000UL
#define UART_VA 0xfffff000UL /* below PHYS */
enum { UART_DR = 0x00, UART_FR = 0x18, UART_FR_RXFE = 1 << 4, UART_FR_TXFF = 1 << 5 };
static volatile uint32_t *uart;

/*
 * QEMU virt's PL031 RTC, 15 pages after the UART: its data register counts
 * seconds since 1970. The PAL reads it once, for the executive's system
 * time (RPB$L_BOOTTIME).
 */
#define RTC_PADDR 0x09010000UL
#define RTC_VA 0xffffe000UL
static uint32_t boot_time;

static void uart_putc(char c)
{
	if (!uart)
		return;
	if (c == '\n')
		uart_putc('\r');
	while (uart[UART_FR / 4] & UART_FR_TXFF)
		;
	uart[UART_DR / 4] = (uint8_t)c;
}

/* Whether the UART has a character the console hasn't read. */
static int uart_rx_ready(void)
{
	return uart && !(uart[UART_FR / 4] & UART_FR_RXFE);
}

static void putnum(unsigned long v, unsigned base)
{
	char buf[20];
	int n = 0;
	do {
		buf[n++] = "0123456789abcdef"[v % base];
		v /= base;
	} while (v);
	while (n)
		uart_putc(buf[--n]);
}

/* printf subset: %s, %c, %u, %x, %lu, %lx, %%. */
static void __attribute__((format(printf, 1, 2))) print(const char *fmt, ...)
{
	va_list ap;
	va_start(ap, fmt);
	for (; *fmt; fmt++) {
		if (*fmt != '%') {
			uart_putc(*fmt);
			continue;
		}
		int wide = *++fmt == 'l';
		fmt += wide;
		if (*fmt == '%')
			uart_putc('%');
		else if (*fmt == 's')
			for (const char *s = va_arg(ap, const char *); *s; s++)
				uart_putc(*s);
		else if (*fmt == 'c')
			uart_putc(va_arg(ap, int));
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

int memcmp(const void *a, const void *b, seL4_Word n)
{
	const unsigned char *x = a, *y = b;
	for (; n; n--, x++, y++)
		if (*x != *y)
			return *x - *y;
	return 0;
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

/* A copy of cap, with a badge if it is an endpoint or notification and
 * badge isn't 0. */
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
 * A frame for the device page at paddr, from the device untyped that holds
 * it, or 0 if none does. Retype carves an untyped in order, from where it
 * last stopped, so untypeds, each aligned and as large as fits, take up
 * the gap before the page.
 * ponytail: the pages of one untyped must come in increasing order.
 */
static seL4_CPtr device_frame(seL4_BootInfo *bi, seL4_Word paddr)
{
	static seL4_Word carved[CONFIG_MAX_NUM_BOOTINFO_UNTYPED_CAPS];
	for (seL4_Word i = 0; i < bi->untyped.end - bi->untyped.start; i++) {
		seL4_UntypedDesc *u = &bi->untypedList[i];
		seL4_Word off = paddr - u->paddr;
		if (!u->isDevice || paddr < u->paddr || off >> u->sizeBits)
			continue;
		if (off < carved[i])
			die("device pages out of order", paddr);
		seL4_CPtr ram = untyped;
		untyped = bi->untyped.start + i;
		while (carved[i] < off) {
			int bit = seL4_PageBits;
			while (!(carved[i] & ((2UL << bit) - 1)) && carved[i] + (2UL << bit) <= off)
				bit++;
			alloc(seL4_UntypedObject, bit);
			carved[i] += 1UL << bit;
		}
		seL4_CPtr frame = alloc(seL4_ARM_SmallPageObject, 0);
		carved[i] += PAGE_SIZE;
		untyped = ram;
		return frame;
	}
	return 0;
}

/* Maps the UART's page, then the RTC's, and reads the RTC. */
static seL4_CPtr uart_frame;

static void uart_init(seL4_BootInfo *bi)
{
	uart_frame = device_frame(bi, UART_PADDR);
	seL4_CPtr rtc_frame = device_frame(bi, RTC_PADDR);
	if (!uart_frame || !rtc_frame)
		return;
	map(uart_frame, seL4_CapInitThreadVSpace, UART_VA, seL4_ReadWrite, seL4_ARM_ExecuteNever);
	uart = (volatile uint32_t *)UART_VA;
	map(rtc_frame, seL4_CapInitThreadVSpace, RTC_VA, seL4_CanRead, seL4_ARM_ExecuteNever);
	boot_time = *(volatile uint32_t *)RTC_VA;
}

/*
 * The disks: virtio block devices on QEMU virt's 32 virtio-mmio
 * transports, 0x200 bytes apart from 0x0a000000, which the PAL drives
 * itself, polled, a request at a time (virtio 1.x, modern MMIO: QEMU needs
 * virtio-mmio.force-legacy=false). Unit 0 is the system disk, DKA0:, and
 * unit 1 the data disk, DKB0:, in the order of QEMU's -device options,
 * which take the transports from the highest down. Each unit's queue and
 * a request's header and status are in a page of the PAL's, the data of
 * both in another, at the physical addresses seL4 tells.
 * ponytail: polled and synchronous, the CPU waits for each request; the
 * device's interrupt, through seL4's IRQ handler, when I/O is
 * asynchronous.
 */
#define VIRTIO_PADDR 0x0a000000UL
#define VIRTIO_VA 0xffff0000UL /* 4 pages, below the RTC */
#define VIRTIO_SLOTS 32
#define DISK_BLOCK 512
#define DISK_UNITS 2
enum { VIO_MAGIC = 0x000, VIO_VERSION = 0x004, VIO_DEVICE = 0x008, VIO_DRVFEAT = 0x020,
       VIO_DRVFEATSEL = 0x024, VIO_QSEL = 0x030, VIO_QNUMMAX = 0x034, VIO_QNUM = 0x038,
       VIO_QREADY = 0x044, VIO_QNOTIFY = 0x050, VIO_INTSTATUS = 0x060, VIO_INTACK = 0x064,
       VIO_STATUS = 0x070, VIO_QDESC = 0x080, VIO_QDRIVER = 0x090, VIO_QDEVICE = 0x0a0,
       VIO_CAPACITY = 0x100 };
enum { VIO_ACK = 1, VIO_DRIVER = 2, VIO_DRIVER_OK = 4, VIO_FEATURES_OK = 8, VIO_NET = 1, VIO_BLK = 2,
       VIO_QSIZE = 4, VIOD_NEXT = 1, VIOD_WRITE = 2 };
struct vio_queue {
	struct {
		uint64_t addr;
		uint32_t len;
		uint16_t flags, next;
	} desc[VIO_QSIZE];
	struct {
		uint16_t flags, idx, ring[VIO_QSIZE];
	} avail __attribute__((aligned(64)));
	struct {
		uint16_t flags, idx;
		struct {
			uint32_t id, len;
		} ring[VIO_QSIZE];
	} used __attribute__((aligned(64)));
	struct {
		uint32_t type, reserved;
		uint64_t sector;
	} req __attribute__((aligned(64)));
	uint8_t status;
};
static struct disk {
	volatile uint32_t *vio; /* 0: no such unit */
	uint64_t blocks;
	struct vio_queue *vq;
} disks[DISK_UNITS];
static uint8_t *vio_data; /* a page: 8 blocks */
static seL4_CPtr vio_frame[VIRTIO_SLOTS * 0x200 / 4096]; /* the transports' pages */
static int net_slot = -1; /* the virtio-net device's transport, or -1 */
#define VQ_VA 0xfffe0000UL /* a page per unit */
#define VIO_DATA_VA 0xfffd0000UL

/* A page of the PAL's for the device, mapped at va, and its physical address. */
static uint64_t dma_page(seL4_Word va)
{
	seL4_CPtr frame = alloc(seL4_ARM_SmallPageObject, 0);
	map(frame, seL4_CapInitThreadVSpace, va, seL4_ReadWrite,
	    seL4_ARM_Default_VMAttributes | seL4_ARM_ExecuteNever);
	seL4_ARM_Page_GetAddress_t a = seL4_ARM_Page_GetAddress(frame);
	if (a.error)
		die("seL4_ARM_Page_GetAddress failed", a.error);
	memset((void *)va, 0, PAGE_SIZE);
	return a.paddr;
}

static void wr64(volatile uint32_t *vio, unsigned reg, uint64_t v)
{
	vio[reg / 4] = (uint32_t)v;
	vio[reg / 4 + 1] = v >> 32;
}

/* Sets up the block device at vio as unit u, whose data goes through the
 * page at data. */
static void disk_setup(unsigned u, volatile uint32_t *vio, uint64_t data)
{
	vio[VIO_STATUS / 4] = 0;
	vio[VIO_STATUS / 4] = VIO_ACK | VIO_DRIVER;
	vio[VIO_DRVFEATSEL / 4] = 1; /* VIRTIO_F_VERSION_1, and nothing else */
	vio[VIO_DRVFEAT / 4] = 1;
	vio[VIO_STATUS / 4] = VIO_ACK | VIO_DRIVER | VIO_FEATURES_OK;
	if (!(vio[VIO_STATUS / 4] & VIO_FEATURES_OK) || vio[VIO_QNUMMAX / 4] < VIO_QSIZE)
		return;
	seL4_Word va = VQ_VA + u * PAGE_SIZE;
	uint64_t q = dma_page(va);
	struct vio_queue *vq = (struct vio_queue *)va;
	vio[VIO_QSEL / 4] = 0;
	vio[VIO_QNUM / 4] = VIO_QSIZE;
	wr64(vio, VIO_QDESC, q + __builtin_offsetof(struct vio_queue, desc));
	wr64(vio, VIO_QDRIVER, q + __builtin_offsetof(struct vio_queue, avail));
	wr64(vio, VIO_QDEVICE, q + __builtin_offsetof(struct vio_queue, used));
	vio[VIO_QREADY / 4] = 1;
	vio[VIO_STATUS / 4] = VIO_ACK | VIO_DRIVER | VIO_FEATURES_OK | VIO_DRIVER_OK;
	/* The request: its header, the data and the status, a chain of three. */
	vq->desc[0] = (typeof(vq->desc[0])){ q + __builtin_offsetof(struct vio_queue, req), 16,
					     VIOD_NEXT, 1 };
	vq->desc[1] = (typeof(vq->desc[0])){ data, 0, VIOD_NEXT, 2 };
	vq->desc[2] = (typeof(vq->desc[0])){ q + __builtin_offsetof(struct vio_queue, status), 1,
					     VIOD_WRITE, 0 };
	disks[u] = (struct disk){ vio, vio[VIO_CAPACITY / 4] | (uint64_t)vio[VIO_CAPACITY / 4 + 1] << 32,
				  vq };
	print("disk %u: virtio-blk, %lu blocks\n", u, (seL4_Word)disks[u].blocks);
}

/* Finds the disks among the virtio-mmio transports and sets them up, and
 * the network device, for the TCP/IP component. */
static void disk_init(seL4_BootInfo *bi)
{
	for (unsigned p = 0; p < VIRTIO_SLOTS * 0x200 / PAGE_SIZE; p++) {
		seL4_CPtr frame = vio_frame[p] = device_frame(bi, VIRTIO_PADDR + p * PAGE_SIZE);
		if (!frame)
			return;
		map(frame, seL4_CapInitThreadVSpace, VIRTIO_VA + p * PAGE_SIZE, seL4_ReadWrite,
		    seL4_ARM_ExecuteNever);
	}
	uint64_t data = dma_page(VIO_DATA_VA);
	vio_data = (uint8_t *)VIO_DATA_VA;
	unsigned u = 0;
	for (unsigned s = VIRTIO_SLOTS; s--;) {
		volatile uint32_t *r = (volatile uint32_t *)(VIRTIO_VA + s * 0x200);
		if (r[VIO_MAGIC / 4] != 0x74726976 || r[VIO_VERSION / 4] != 2)
			continue;
		if (r[VIO_DEVICE / 4] == VIO_BLK && u < DISK_UNITS)
			disk_setup(u++, r, data);
		else if (r[VIO_DEVICE / 4] == VIO_NET && net_slot < 0)
			net_slot = s;
	}
}

/* Reads (write 0) or writes (1) n blocks, at most 8, of unit d at lbn,
 * from or to vio_data. Returns 0 on an error. */
static int disk_io8(struct disk *d, uint64_t lbn, unsigned n, int write)
{
	struct vio_queue *vq = d->vq;
	vq->req.type = write; /* VIRTIO_BLK_T_IN, VIRTIO_BLK_T_OUT */
	vq->req.sector = lbn;
	vq->desc[1].len = n * DISK_BLOCK;
	vq->desc[1].flags = write ? VIOD_NEXT : VIOD_NEXT | VIOD_WRITE;
	vq->status = 0xff;
	uint16_t idx = vq->avail.idx;
	vq->avail.ring[idx % VIO_QSIZE] = 0;
	__asm__ volatile("dmb sy" ::: "memory");
	vq->avail.idx = idx + 1;
	__asm__ volatile("dmb sy" ::: "memory");
	d->vio[VIO_QNOTIFY / 4] = 0;
	while (*(volatile uint16_t *)&vq->used.idx != (uint16_t)(idx + 1))
		;
	__asm__ volatile("dmb sy" ::: "memory");
	d->vio[VIO_INTACK / 4] = d->vio[VIO_INTSTATUS / 4];
	return vq->status == 0;
}

/*
 * Reads n blocks of the system disk from lbn into the PAL's memory at dst.
 * Returns 0 if there is no disk, the blocks run past its end or the device
 * fails.
 */
static int disk_read(uint64_t lbn, uint8_t *dst, seL4_Word n)
{
	struct disk *d = &disks[0];
	if (!d->vio || lbn + n > d->blocks)
		return 0;
	while (n) {
		unsigned k = n < PAGE_SIZE / DISK_BLOCK ? n : PAGE_SIZE / DISK_BLOCK;
		if (!disk_io8(d, lbn, k, 0))
			return 0;
		memcpy(dst, vio_data, k * DISK_BLOCK);
		dst += k * DISK_BLOCK, lbn += k, n -= k;
	}
	return 1;
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
static seL4_CPtr pfn_frame[PFN_COUNT];	 /* mapped at PHYS */
static seL4_CPtr pfn_cap[PFN_COUNT];	 /* mapped in the executive's kernel mode */
static seL4_CPtr pfn_mcap[4][PFN_COUNT]; /* in its process's modes 1-3, made as needed */
static seL4_Word pfn_va[PFN_COUNT];	 /* where, 0 if nowhere */
static struct ctx *pfn_ctx[PFN_COUNT];	 /* whose P0 or P1, 0 for S0 */

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
 * The executive's address space, below 2 GB, where MACRO-32's longwords
 * reach, in the VAX's three regions: P0 and P1, each process's own, and
 * between them S0, the system space every process shares. The PAL keeps the
 * PTE it was last given for each page, 4 MB of them to a table: S0's in one
 * directory, each process's P0 and P1 in its context's.
 * ponytail: 96 tables, enough for S0 and two per process; more if needed.
 */
#define P0_END 0x40000000UL
#define S0_END 0x60000000UL
#define SPACE_END 0x80000000UL
enum { PTE_VALID = 1u << 31, PTE_EXEC = 1u << 25, PTE_PFN = 0x1fffff, PRT_NA = 0, PRT_KW = 2,
       PRT_KR = 3 };
/* By VAX protection code ($PRTDEF), the outermost mode that may read and
 * the outermost that may write: 0 kernel, 1 executive, 2 supervisor, 3 user,
 * -1 none. Code 1 is reserved. */
static const signed char may_read[16] = { -1, -1, 0, 0, 3, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3 };
static const signed char may_write[16] = { -1, -1, 0, -1, 3, 1, 0, -1, 2, 1, 0, -1, 2, 1, 0, -1 };
static seL4_CPtr exec_vspace;
static uint32_t *s0_dir[SPACE_END >> 22];
static uint32_t pte_pool[96][1024];
static uint32_t *pte_free[96];
static unsigned pte_used, pte_nfree;

static int is_s0(seL4_Word va)
{
	return va >= P0_END && va < S0_END;
}

static unsigned prot(uint32_t pte)
{
	return pte >> 27 & 15;
}

static int mapped(uint32_t pte)
{
	return pte & PTE_VALID && prot(pte) != PRT_NA;
}

/*
 * A process context: the threads that run one executive process, made the
 * first time the executive switches to its hardware PCB, and its P0 and P1.
 * Its seL4 objects outlive it, for the next context in the slot.
 */
#define CTX_MAX 32
static struct ctx {
	seL4_Word hwpcb; /* the executive's HWPCB address; 0: a free slot */
	/* A thread for each mode it has entered, and the mode's address space:
	 * the executive's for kernel mode, one of its own for each other. */
	seL4_CPtr tcb[4], vspace[4], reply;
	int started, mode, prvmode; /* the PSL's current and previous modes */
	uint32_t *dir[SPACE_END >> 22]; /* its P0 and P1 PTE tables */
	/* Its PAL call's message while it waits in one: x0-x7, the svc's PC. */
	seL4_Word mr[seL4_UnknownSyscall_FaultIP + 1];
} ctx[CTX_MAX];
static struct ctx *cur;

/* The PTE of va's page: in S0's directory, or the current process's. */
static uint32_t *pte_at(seL4_Word va, int make)
{
	uint32_t **t = is_s0(va) ? &s0_dir[va >> 22] : cur ? &cur->dir[va >> 22] : 0;
	if (!t)
		return 0;
	if (!*t && make) {
		if (pte_nfree)
			*t = pte_free[--pte_nfree];
		else if (pte_used < sizeof pte_pool / sizeof pte_pool[0])
			*t = pte_pool[pte_used++];
		else
			die("out of PTE tables", va);
	}
	return *t ? &(*t)[va >> seL4_PageBits & 1023] : 0;
}

/* Maps a page as pte says in vspace, with what mode m may do with it. */
static void map_pte(seL4_CPtr frame, seL4_CPtr vspace, seL4_Word va, uint32_t pte, int m)
{
	seL4_CapRights_t rights = m <= may_write[prot(pte)] ? seL4_ReadWrite : seL4_CanRead;
	seL4_ARM_VMAttributes attr = seL4_ARM_Default_VMAttributes;
	if (!(pte & PTE_EXEC))
		attr |= seL4_ARM_ExecuteNever;
	map(frame, vspace, va, rights, attr);
}

static void unmap(seL4_CPtr frame)
{
	seL4_Error err = frame ? seL4_ARM_Page_Unmap(frame) : seL4_NoError;
	if (err)
		die("seL4_ARM_Page_Unmap failed", err);
}

/* The frame cap that maps pfn in its process's mode m address space. */
static seL4_CPtr mode_cap(int m, unsigned pfn)
{
	if (!pfn_mcap[m][pfn])
		pfn_mcap[m][pfn] = mint(pfn_frame[pfn], 0);
	return pfn_mcap[m][pfn];
}

/*
 * Maps a P0 or P1 page of c's in the address spaces of each mode it has
 * entered that may read it, or unmaps it from all of them. Kernel mode's
 * is the executive's, which holds only the current process's pages.
 */
static void map_page(struct ctx *c, seL4_Word va, uint32_t pte, int on)
{
	unsigned pfn = pte & PTE_PFN;
	for (int m = 0; m < 4; m++) {
		if (!on) {
			unmap(m ? pfn_mcap[m][pfn] : pfn_cap[pfn]);
			continue;
		}
		if (!c->tcb[m] || m > may_read[prot(pte)] || (!m && c != cur))
			continue;
		map_pte(m ? mode_cap(m, pfn) : pfn_cap[pfn], c->vspace[m], va, pte, m);
	}
}

/*
 * The S0 pages an outer mode may read, such as the system service vector:
 * each mode's address space of each process maps them when it is made.
 * ponytail: their protection is set before any outer mode runs, and stays.
 */
static struct {
	seL4_Word va;
	uint32_t pte;
} shared[8];
static unsigned nshared;
static int outer_started;

static int outer(uint32_t pte)
{
	return mapped(pte) && may_read[prot(pte)] > 0;
}

/*
 * WRPTE: makes va's page what pte says, mapped, unmapped or with a new
 * protection, and puts the PTE it had in *old. A page in P0 or P1 is the
 * current process's. Fails, returning 0, for an address outside the
 * executive's space, a reserved protection code, a PFN out of range or
 * already mapped at another address, or an S0 page outer modes may read once
 * one of them runs.
 * ponytail: one mapping per PFN; a frame cap per extra mapping for shared
 * pages.
 */
static int wrpte(seL4_Word va, uint32_t pte, uint32_t *old)
{
	unsigned pfn = pte & PTE_PFN;
	int s0 = is_s0(va);
	struct ctx *owner = s0 ? 0 : cur;
	if (va % PAGE_SIZE || va < PAGE_SIZE || va >= SPACE_END || (!s0 && !cur))
		return 0;
	if (mapped(pte) && (prot(pte) == 1 || pfn >= PFN_COUNT ||
			    (pfn_va[pfn] && (pfn_va[pfn] != va || pfn_ctx[pfn] != owner))))
		return 0;
	uint32_t none = 0, *slot = pte_at(va, mapped(pte));
	if (!slot)
		slot = &none; /* unmapping a page in no table: nothing there */
	if (s0 && (outer(pte) || outer(*slot)) &&
	    (outer_started || (!outer(*slot) && nshared == sizeof shared / sizeof shared[0])))
		return 0;
	*old = *slot;
	if (mapped(*old)) {
		unsigned o = *old & PTE_PFN;
		if (s0)
			unmap(pfn_cap[o]);
		else
			map_page(cur, va, *old, 0);
		pfn_va[o] = 0;
		pfn_ctx[o] = 0;
	}
	*slot = pte;
	if (s0) {
		unsigned i = 0;
		while (i < nshared && shared[i].va != va)
			i++;
		if (outer(pte)) {
			shared[i].va = va;
			shared[i].pte = pte;
			nshared += i == nshared;
		} else if (i < nshared) {
			shared[i] = shared[--nshared];
		}
	}
	if (!mapped(pte))
		return 1;
	pfn_ptr(pfn);
	if (s0)
		map_pte(pfn_cap[pfn], exec_vspace, va, pte, 0);
	else
		map_page(cur, va, pte, 1);
	if (pte & PTE_EXEC) {
		seL4_Error err = seL4_ARM_Page_Unify_Instruction(pfn_cap[pfn], 0, PAGE_SIZE);
		if (err)
			die("seL4_ARM_Page_Unify_Instruction failed", err);
	}
	pfn_va[pfn] = va;
	pfn_ctx[pfn] = owner;
	return 1;
}

/*
 * The executive's kernel mode sees the current process's P0 and P1: on a
 * switch, the PAL unmaps the old one's pages there (on = 0) and maps the new
 * one's. seL4 can't swap a page table in and out, since unmapping one clears
 * it (ADR-0005).
 * ponytail: a seL4 call per page of the two processes; an address space of
 * its own per process, with S0 in each, when switches cost too much.
 */
static void kernel_space(struct ctx *c, int on)
{
	for (seL4_Word i = 0; i < SPACE_END >> 22; i++)
		for (unsigned j = 0; c->dir[i] && j < 1024; j++) {
			uint32_t pte = c->dir[i][j];
			if (!mapped(pte))
				continue;
			if (on)
				map_pte(pfn_cap[pte & PTE_PFN], exec_vspace,
					i << 22 | j << seL4_PageBits, pte, 0);
			else
				unmap(pfn_cap[pte & PTE_PFN]);
		}
}

/* Takes back c's P0 and P1 when its context is deleted: the pages the
 * executive left there, and the tables. */
static void free_space(struct ctx *c)
{
	for (seL4_Word i = 0; i < SPACE_END >> 22; i++) {
		uint32_t *t = c->dir[i];
		if (!t)
			continue;
		for (unsigned j = 0; j < 1024; j++)
			if (mapped(t[j])) {
				map_page(c, i << 22 | j << seL4_PageBits, t[j], 0);
				pfn_va[t[j] & PTE_PFN] = 0;
				pfn_ctx[t[j] & PTE_PFN] = 0;
			}
		memset(t, 0, 1024 * sizeof *t);
		pte_free[pte_nfree++] = t;
		c->dir[i] = 0;
	}
}

/* The executive's quadword at va, which must be 8-byte aligned and mapped. */
static uint64_t *quad(seL4_Word va)
{
	uint32_t *pte = va % 8 || va >= SPACE_END ? 0 : pte_at(va, 0);
	if (!pte || !mapped(*pte))
		return 0;
	return (uint64_t *)(PHYS + (*pte & PTE_PFN) * PAGE_SIZE + va % PAGE_SIZE);
}

/*
 * READLBLK and WRITELBLK: read the blocks of unit u from lbn into the len
 * bytes at va, or write those bytes there, as Alpha's console READ and
 * WRITE callbacks did for VMS's bootstrap. Kernel mode must be able to
 * write (read) the buffer. Return an SS$ status: NORMAL, ACCVIO for a
 * buffer kernel mode can't, ILLBLKNUM past the disk's end, NOSUCHDEV
 * without the disk, DRVERR if the device fails, as it does for a write to
 * the system disk, which QEMU attaches read only.
 */
enum { SS_NORMAL = 1, SS_ACCVIO = 12, SS_DRVERR = 140, SS_ILLBLKNUM = 220, SS_NOSUCHDEV = 2312 };

static seL4_Word lblk(seL4_Word va, seL4_Word len, seL4_Word lbn, seL4_Word u, int write)
{
	uint8_t *at[2 + 0xffff / PAGE_SIZE]; /* the PAL's view of each page */
	seL4_Word first = va & ~(PAGE_SIZE - 1);
	for (seL4_Word p = first, i = 0; p < va + len; p += PAGE_SIZE, i++) {
		uint32_t *pte = p < SPACE_END ? pte_at(p, 0) : 0;
		if (!pte || !mapped(*pte) || (!write && may_write[prot(*pte)] < 0))
			return SS_ACCVIO;
		at[i] = (uint8_t *)(PHYS + (*pte & PTE_PFN) * PAGE_SIZE);
	}
	if (u >= DISK_UNITS || !disks[u].vio)
		return SS_NOSUCHDEV;
	struct disk *d = &disks[u];
	seL4_Word blocks = (len + DISK_BLOCK - 1) / DISK_BLOCK;
	if (lbn + blocks > d->blocks)
		return SS_ILLBLKNUM;
	for (seL4_Word done = 0; done < len;) {
		unsigned k = blocks < 8 ? blocks : 8;
		seL4_Word n = len - done < k * DISK_BLOCK ? len - done : k * DISK_BLOCK;
		if (write) /* a last block in part is padded with zeros */
			memset(vio_data, 0, k * DISK_BLOCK);
		for (seL4_Word i = 0; write && i < n; i++) {
			seL4_Word off = va + done + i - first;
			vio_data[i] = at[off / PAGE_SIZE][off % PAGE_SIZE];
		}
		if (!disk_io8(d, lbn, k, write))
			return SS_DRVERR;
		for (seL4_Word i = 0; !write && i < n; i++) {
			seL4_Word off = va + done + i - first;
			at[off / PAGE_SIZE][off % PAGE_SIZE] = vio_data[i];
		}
		done += n, lbn += k, blocks -= k;
	}
	return SS_NORMAL;
}

/*
 * PROBER and PROBEW: whether mode m, or the PSL's previous mode if it is an
 * outer one, may read (write) the first and the last of len bytes at base,
 * as the VAX's PROBE checks.
 */
static int probe(seL4_Word base, seL4_Word len, int m, int write)
{
	if (m < cur->prvmode)
		m = cur->prvmode;
	seL4_Word at[2] = { base, base + (len ? len - 1 : 0) };
	for (int i = 0; i < 2; i++) {
		uint32_t *pte = at[i] < SPACE_END ? pte_at(at[i], 0) : 0;
		if (!pte || !mapped(*pte) || m > (write ? may_write : may_read)[prot(*pte)])
			return 0;
	}
	return 1;
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
 * Files-11 ODS-2 (ods/docs/), as much of it as VMS's primary bootstrap,
 * VMB, read to find the executive on the system disk: the home block, the
 * index file's header, the directories on the way and the file's map.
 */
static uint8_t f11_indexf[DISK_BLOCK], f11_blk[DISK_BLOCK];
static uint32_t f11_hdrvbn; /* the index file's VBN of file 1's header */

static uint16_t u16(const uint8_t *p)
{
	return p[0] | p[1] << 8;
}

/* The LBN of virtual block vbn of the file whose header is hdr, and in *n
 * how many blocks follow it in the same extent; 0 if the map ends first. */
static uint32_t f11_map(const uint8_t *hdr, uint32_t vbn, uint32_t *n)
{
	const uint8_t *p = hdr + 2 * hdr[1], *end = p + 2 * hdr[58];
	for (uint32_t base = 1; p < end;) {
		uint32_t w = u16(p), count, lbn;
		switch (w >> 14) {
		case 0:
			p += 2;
			continue;
		case 1:
			count = (w & 0xff) + 1, lbn = (w >> 8 & 0x3f) << 16 | u16(p + 2), p += 4;
			break;
		case 2:
			count = (w & 0x3fff) + 1, lbn = u32(p + 2), p += 6;
			break;
		default:
			count = ((w & 0x3fff) << 16 | u16(p + 2)) + 1, lbn = u32(p + 4), p += 8;
			break;
		}
		if (vbn < base + count) {
			*n = base + count - vbn;
			return lbn + vbn - base;
		}
		base += count;
	}
	return 0;
}

/* Reads n blocks of hdr's file from vbn into dst. Returns 0 on an error. */
static int f11_read(const uint8_t *hdr, uint32_t vbn, uint8_t *dst, uint32_t n)
{
	while (n) {
		uint32_t k, lbn = f11_map(hdr, vbn, &k);
		if (!lbn)
			return 0;
		k = k < n ? k : n;
		if (!disk_read(lbn, dst, k))
			return 0;
		dst += k * DISK_BLOCK, vbn += k, n -= k;
	}
	return 1;
}

/* The header of file number fnum, in hdr. Returns 0 if it isn't one. */
static int f11_header(uint32_t fnum, uint8_t *hdr)
{
	uint16_t sum = 0;
	if (!f11_read(f11_indexf, f11_hdrvbn + fnum - 1, hdr, 1))
		return 0;
	for (int i = 0; i < 255; i++)
		sum += u16(hdr + 2 * i);
	return sum == u16(hdr + 510) && (uint32_t)(u16(hdr + 8) | hdr[13] << 16) == fnum;
}

/* The end of file of hdr's file: its blocks in use, the last maybe partly. */
static uint32_t f11_eof(const uint8_t *hdr, uint32_t *ffbyte)
{
	*ffbyte = u16(hdr + 32);
	return (uint32_t)u16(hdr + 28) << 16 | u16(hdr + 30); /* high word first */
}

/* The file number of name, "NAME.TYPE", its highest version, in the
 * directory whose number is dir; 0 if there is none. */
static uint32_t f11_lookup(uint32_t dir, const char *name)
{
	uint8_t hdr[DISK_BLOCK];
	uint32_t ff, len = 0;
	while (name[len])
		len++;
	if (!f11_header(dir, hdr))
		return 0;
	for (uint32_t vbn = 1, eof = f11_eof(hdr, &ff); vbn < eof + (ff != 0); vbn++) {
		if (!f11_read(hdr, vbn, f11_blk, 1))
			return 0;
		for (uint32_t off = 0; off < DISK_BLOCK - 2 && u16(f11_blk + off) != 0xffff;
		     off += 2 + u16(f11_blk + off)) {
			const uint8_t *r = f11_blk + off;
			uint32_t i = 0;
			while (i < len && i < r[5] && r[6 + i] == name[i])
				i++;
			if (i == len && r[5] == len) {
				const uint8_t *e = r + 6 + ((len + 1) & ~1u);
				return u16(e + 2) | e[7] << 16;
			}
		}
	}
	return 0;
}

/* Mounts the system disk and reads [SYSEXE]EXEC.EXE into buf, at most size
 * bytes. Returns its size, or 0. */
static seL4_Word f11_boot_file(uint8_t *buf, seL4_Word size)
{
	enum { HM2_IBMAPVBN = 22, HM2_IBMAPLBN = 24, HM2_IBMAPSIZE = 32, HM2_FORMAT = 496, MFD = 4 };
	uint8_t *home = f11_blk, hdr[DISK_BLOCK];
	if (!disk_read(1, home, 1) || memcmp(home + HM2_FORMAT, "DECFILE11B", 10))
		return 0;
	f11_hdrvbn = u16(home + HM2_IBMAPVBN) + u16(home + HM2_IBMAPSIZE);
	if (!disk_read(u32(home + HM2_IBMAPLBN) + u16(home + HM2_IBMAPSIZE), f11_indexf, 1))
		return 0;
	uint32_t dir = f11_lookup(MFD, "SYSEXE.DIR"), file = dir ? f11_lookup(dir, "EXEC.EXE") : 0;
	uint32_t ff, eof;
	if (!file || !f11_header(file, hdr))
		return 0;
	eof = f11_eof(hdr, &ff);
	seL4_Word bytes = (seL4_Word)(eof - 1) * DISK_BLOCK + ff;
	if (!eof || bytes > size || !f11_read(hdr, 1, buf, (bytes + DISK_BLOCK - 1) / DISK_BLOCK))
		return 0;
	return bytes;
}

/*
 * What the PAL sets up in the executive's address space before it starts
 * (DESIGN-0001), in S0: the restart parameter block and a stack;
 * EXEC.EXE's sections are at their link addresses, in S0 too.
 */
#define RPB_VA 0x4fff0000UL
#define PORT_VA 0x4ff00000UL /* the port's pages, if there is a network device */
#define EXEC_STACK_TOP 0x5fff0000UL
#define EXEC_STACK_PAGES 4
enum { RPB_BASE = 0, RPB_PFNCNT = 4, RPB_FREEPFN = 8, RPB_BOOTTIME = 12, RPB_PORT = 16,
       RPB_HWPCB = 64,
       RPB_LENGTH = 192 };

static seL4_CPtr fault_ep, sched_control;
static seL4_Time slice;
/* The current context waits in WTINT, its reply object taken: the PAL
 * receives on idle_reply meanwhile. */
static int idle;
static seL4_CPtr idle_reply;
/* The clock ticks every TICK_US, signalling the PAL with CLOCK_BADGE. */
#define TICK_US 10000
#define CLOCK_BADGE (1UL << 32)
/* The TCP/IP component signals the PAL's notification with PORT_BADGE,
 * and faults to its endpoint with TCPIP_BADGE. */
#define PORT_BADGE (1UL << 33)
#define TCPIP_BADGE (1UL << 34)

static struct ctx *find_ctx(seL4_Word hwpcb)
{
	for (struct ctx *c = ctx; c < ctx + CTX_MAX; c++)
		if (c->hwpcb == hwpcb)
			return c;
	return 0;
}

/* What the PAL's endpoint tells a context's threads by: its slot and mode. */
static seL4_Word ctx_badge(struct ctx *c, int m)
{
	return (seL4_Word)(c - ctx + 1) | (seL4_Word)m << 8;
}

/*
 * A thread in vspace with an empty CSpace, since it calls nothing but the
 * PAL, and the PAL's endpoint, badged, for its faults, PAL calls included.
 * It runs below the PAL's priority, so the PAL serves a call as soon as it
 * is made, and the TCP/IP component's, so a doorbell is answered at once.
 */
static seL4_CPtr new_thread(seL4_CPtr vspace, seL4_Word badge)
{
	seL4_CPtr tcb = alloc(seL4_TCBObject, 0);
	seL4_CPtr sc = alloc(seL4_SchedContextObject, seL4_MinSchedContextBits);
	seL4_Error err = seL4_TCB_Configure(tcb, alloc(seL4_CapTableObject, 1), 0, vspace, 0, 0,
					    seL4_CapNull);
	if (!err)
		err = seL4_SchedControl_Configure(sched_control, sc, slice, slice, 0, 0);
	if (!err)
		err = seL4_TCB_SetSchedParams(tcb, seL4_CapInitThreadTCB, seL4_MaxPrio - 2,
					      seL4_MaxPrio - 2, sc, mint(fault_ep, badge));
	if (err)
		die("configuring a process context failed", err);
	return tcb;
}

/* A free slot for hwpcb, whose kernel-mode thread runs in the executive's
 * address space. */
static struct ctx *new_ctx(seL4_Word hwpcb)
{
	struct ctx *c = find_ctx(0);
	if (!c)
		return 0;
	if (!c->tcb[0]) {
		c->vspace[0] = exec_vspace;
		c->tcb[0] = new_thread(exec_vspace, ctx_badge(c, 0));
		c->reply = alloc(seL4_ReplyObject, 0);
	}
	c->hwpcb = hwpcb;
	c->started = c->mode = c->prvmode = 0;
	return c;
}

/*
 * The context's thread for outer mode m, made the first time it enters
 * that mode, in an address space of its own: the process's pages m may
 * read, and the shared S0 ones.
 */
static void mode_thread(struct ctx *c, int m)
{
	if (c->tcb[m])
		return;
	c->vspace[m] = alloc(seL4_ARM_VSpaceObject, 0);
	seL4_Error err = seL4_ARM_ASIDPool_Assign(seL4_CapInitThreadASIDPool, c->vspace[m]);
	if (err)
		die("seL4_ARM_ASIDPool_Assign failed", err);
	c->tcb[m] = new_thread(c->vspace[m], ctx_badge(c, m));
	outer_started = 1;
	for (unsigned i = 0; i < nshared; i++)
		if (m <= may_read[prot(shared[i].pte)])
			map_pte(mint(pfn_frame[shared[i].pte & PTE_PFN], 0), c->vspace[m],
				shared[i].va, shared[i].pte, m);
	for (seL4_Word i = 0; i < SPACE_END >> 22; i++)
		for (unsigned j = 0; c->dir[i] && j < 1024; j++) {
			uint32_t pte = c->dir[i][j];
			if (mapped(pte) && m <= may_read[prot(pte)])
				map_pte(mode_cap(m, pte & PTE_PFN), c->vspace[m],
					i << 22 | j << seL4_PageBits, pte, m);
		}
}

/* PAL function codes (docs/design/0001-pal-interface.md). */
enum {
	HALT = 0x00, SWPCTX = 0x05, MFPR_IPL = 0x0e, MTPR_IPL = 0x0f, MFPR_PCBB = 0x12,
	MFPR_SCBB = 0x16, MTPR_SCBB = 0x17, MTPR_SIRR = 0x18, MFPR_SISR = 0x19, WTINT = 0x3e,
	MTPR_TXDB = 0x40, WRPTE = 0x41, DELCTX = 0x42, MTPR_RXCS = 0x43, MFPR_RXCS = 0x44,
	MFPR_RXDB = 0x45, READLBLK = 0x46, WRITELBLK = 0x47, MTPR_DOORBELL = 0x48, CHME = 0x82, CHMU = 0x85, PROBER = 0x8f,
	PROBEW = 0x90, RD_PS = 0x91, REI = 0x92
};

/* The system control block's vectors the PAL delivers through, and the
 * interval timer's and console's IPLs, the VAX's. CHMx's is SCB_CHMK + 4 * x,
 * x the mode. */
enum { SCB_OPCDEC = 0x10, SCB_ACCVIO = 0x20, SCB_CHMK = 0x40, SCB_SOFTINT = 0x80,
       SCB_TIMER = 0xc0, SCB_CONSRCV = 0xf8, SCB_PORT = 0x100, IPL_ASTDEL = 2, IPL_CONSOLE = 20,
       IPL_PORT = 21, IPL_HWCLK = 24 };

/* The HWPCB's AST enable and summary bytes, Alpha's ASTEN and ASTSR
 * ($HWPCBDEF): bit n of each is access mode n's. */
enum { HWPCB_ASTEN = 32 };

/* The console receive status register's bits, the VAX's RXCS: a character
 * is waiting (DONE), and interrupt when one is (IE). */
enum { RXCS_DONE = 0x80, RXCS_IE = 0x40 };

/*
 * The processor state the PAL keeps for the executive's one CPU. A VAX
 * starts at IPL 31. pending has a bit per IPL with an interrupt requested
 * and not yet delivered: software interrupts at 1-15, SISR, the console at
 * IPL_CONSOLE, the port's completion at IPL_PORT and the interval timer at
 * IPL_HWCLK. rxcs holds RXCS_IE.
 */
static seL4_Word ipl = 31, pending, scbb, rxcs;

/* The TCP/IP component's notification, badged for a tick and for the
 * doorbell; 0 without the component. */
static seL4_CPtr tcpip_tick, doorbell;

/* Set when the executive halts, rather than faults: the PAL powers off. */
static int halted;

/* The frame on the stack an interrupt or exception pushes and REI pops
 * ($INTSTKDEF), in quadwords: every register of the code it stops. */
enum { F_PC, F_PS, F_X0, F_X_SP = F_X0 + 31, F_LENGTH };

/* Where seL4 keeps xn, n 0-30, in the registers it reads and writes. */
#define X(n) __builtin_offsetof(seL4_UserContext, x##n)
static const unsigned short xoff[31] = { X(0),	X(1),  X(2),  X(3),  X(4),  X(5),  X(6),  X(7),
					 X(8),	X(9),  X(10), X(11), X(12), X(13), X(14), X(15),
					 X(16), X(17), X(18), X(19), X(20), X(21), X(22), X(23),
					 X(24), X(25), X(26), X(27), X(28), X(29), X(30) };
#undef X

static seL4_Word *xreg(seL4_UserContext *r, int n)
{
	return (seL4_Word *)((char *)r + xoff[n]);
}

/* The PSL: the current and previous modes in bits 25:24 and 23:22, IPL in
 * 20:16, and the condition codes, NZVC in 3:0, from ARM64's NZCV in 31:28. */
static seL4_Word psl(seL4_Word spsr)
{
	return (seL4_Word)cur->mode << 24 | (seL4_Word)cur->prvmode << 22 | ipl << 16 |
	       (spsr >> 28 & 0xc) | (spsr >> 29 & 1) | (spsr >> 27 & 2);
}

static seL4_Word spsr(seL4_Word psl)
{
	return (psl & 0xc) << 28 | (psl & 1) << 29 | (psl & 2) << 27;
}

/* The stack pointer of mode m the current HWPCB keeps: KSP, ESP, SSP, USP. */
static uint64_t *hwpcb_sp(int m)
{
	return quad(cur->hwpcb + 8 * m);
}

/*
 * Delivers an interrupt or exception to the handler the SCB has at off, in
 * mode `to`, with prv as the previous mode, and raises IPL to new_ipl. The
 * frame, and n parameters below it, go on the current stack, 16-byte
 * aligned, or, if `to` is an inner mode, on its stack from the HWPCB, where
 * the current mode's stack pointer goes, the lower of VAX SP and sp, so
 * that an AST routine called on it overwrites nothing. Returns 0, having changed nothing,
 * if there is no SCB, no handler or no stack to push on.
 */
static int vector(seL4_UserContext *r, seL4_Word off, seL4_Word new_ipl, int to, int prv,
		  const seL4_Word *param, unsigned n)
{
	uint64_t *v = scbb ? quad((scbb + off) & ~7UL) : 0;
	seL4_Word handler = v ? *v >> (scbb + off) % 8 * 8 & 0xffffffff : 0;
	if (!handler)
		return 0;
	seL4_Word frame[F_LENGTH] = { r->pc, psl(r->spsr) };
	for (int i = 0; i < 31; i++)
		frame[F_X0 + i] = *xreg(r, i);
	frame[F_X_SP] = r->sp;
	/* Below both stacks: vmacro moves sp first, then x18, the VAX SP, and back. */
	seL4_Word sp = r->x18 < r->sp ? r->x18 : r->sp, below = sp;
	uint64_t *outer = 0;
	if (to < cur->mode) {
		uint64_t *inner = hwpcb_sp(to);
		outer = hwpcb_sp(cur->mode);
		if (!inner || !*inner || !outer)
			return 0;
		sp = *inner;
	}
	seL4_Word params = (n * 8 + 15) & ~15UL;
	sp = (sp & ~15UL) - sizeof frame - params;
	uint64_t *q[F_LENGTH + 2];
	for (unsigned i = 0; i < n + F_LENGTH; i++)
		if (!(q[i] = quad(sp + (i < n ? 8 * i : params + 8 * (i - n)))))
			return 0;
	for (unsigned i = 0; i < n + F_LENGTH; i++)
		*q[i] = i < n ? param[i] : frame[i - n];
	if (outer)
		*outer = below;
	r->x18 = r->sp = sp;
	r->pc = handler;
	cur->mode = to;
	cur->prvmode = prv;
	ipl = new_ipl;
	mode_thread(cur, to);
	return 1;
}

/*
 * Requests the AST delivery interrupt, software interrupt IPL_ASTDEL, if
 * IPL is below it and an AST is pending (ASTSR) and enabled (ASTEN) for the
 * current mode or an inner one, as the VAX's REI did with ASTLVL. The
 * executive writes both bytes in the HWPCB, and the PAL reads them each
 * time IPL or the mode may let an AST through.
 */
static void astchk(void)
{
	uint64_t *q = quad(cur->hwpcb + HWPCB_ASTEN);
	seL4_Word ast = q ? *q : 0;
	if (ipl < IPL_ASTDEL && ast & ast >> 8 & ((2UL << cur->mode) - 1))
		pending |= 1UL << IPL_ASTDEL;
}

/* Delivers the highest pending interrupt IPL lets through, in kernel mode,
 * as the VAX's interrupts are, with kernel as the previous mode too. */
static int deliver(seL4_UserContext *r)
{
	astchk();
	for (seL4_Word level = 31; level > ipl; level--)
		if (pending >> level & 1) {
			pending &= ~(1UL << level);
			seL4_Word off = level == IPL_HWCLK     ? SCB_TIMER :
					level == IPL_CONSOLE ? SCB_CONSRCV :
					level == IPL_PORT    ? SCB_PORT :
							       SCB_SOFTINT + 4 * level;
			if (!vector(r, off, level, 0, 0, 0, 0)) {
				print("%%PAL-F-NOVEC, no handler for interrupt at IPL %lu\n", level);
				return 0;
			}
		}
	return 1;
}

static int deliverable(void)
{
	astchk();
	return pending >> (ipl + 1) != 0;
}

/*
 * REI: pops the frame at VAX SP. Returns 0 if it isn't one, or its PSL
 * isn't one REI may load, as on the VAX: no inner mode than the current
 * one, a previous mode no inner than the new one, IPL 0 outside kernel
 * mode. Going out to another mode leaves the current one's stack pointer,
 * past the frame, in the HWPCB.
 */
static int rei(seL4_UserContext *r)
{
	seL4_Word f[F_LENGTH];
	for (unsigned i = 0; i < F_LENGTH; i++) {
		uint64_t *q = quad(r->x18 + 8 * i);
		if (!q)
			return 0;
		f[i] = *q;
	}
	int m = f[F_PS] >> 24 & 3, prv = f[F_PS] >> 22 & 3;
	if (m < cur->mode || prv < m || (m && f[F_PS] >> 16 & 31))
		return 0;
	if (m != cur->mode) {
		uint64_t *sp = hwpcb_sp(cur->mode);
		if (!sp)
			return 0;
		*sp = r->x18 + sizeof f;
		mode_thread(cur, m);
	}
	cur->mode = m;
	cur->prvmode = prv;
	r->pc = f[F_PC];
	r->spsr = spsr(f[F_PS]);
	for (int i = 0; i < 31; i++)
		*xreg(r, i) = f[F_X0 + i];
	r->sp = f[F_X_SP];
	ipl = f[F_PS] >> 16 & 31;
	return 1;
}

/* The CPU's registers, while the PAL works on them, and the mode whose
 * thread they came from. */
static seL4_UserContext regs;
static int regs_mode;

static void read_regs(void)
{
	regs_mode = cur->mode;
	seL4_Error err = seL4_TCB_ReadRegisters(cur->tcb[regs_mode], 0, 0,
						sizeof regs / sizeof(seL4_Word), &regs);
	if (err)
		die("seL4_TCB_ReadRegisters failed", err);
}

/*
 * Resumes the current context with regs, in the thread of its mode: one
 * that is new or stopped is resumed by the write, one in a PAL call by the
 * reply. If the mode changed, the thread regs came from stops, and its PAL
 * call with it.
 */
static void write_regs(int resume)
{
	if (cur->mode != regs_mode) {
		seL4_TCB_Suspend(cur->tcb[regs_mode]);
		resume = 1;
	}
	seL4_Error err = seL4_TCB_WriteRegisters(cur->tcb[cur->mode], resume, 0,
						 sizeof regs / sizeof(seL4_Word), &regs);
	if (err)
		die("seL4_TCB_WriteRegisters failed", err);
	if (!resume)
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
	kernel_space(from, 0);
	cur = to;
	kernel_space(to, 1);
	if (to->started) {
		/* Blocked in its own SWPCTX: ret() replies to it. */
		return ret(from->hwpcb);
	}
	to->started = 1;
	regs = (seL4_UserContext){ .x18 = *ksp };
	regs_mode = 0;
	if (!rei(&regs)) {
		print("%%PAL-F-SWPCTX, no REI frame at KSP 0x%lx\n", (seL4_Word)*ksp);
		return 0;
	}
	regs.x0 = from->hwpcb;
	if (!deliver(&regs))
		return 0;
	write_regs(1);
	return 1;
}

/*
 * An exception of the current context's: delivered in kernel mode at the
 * same IPL, with the PC of the instruction that took it, if it came from an
 * outer mode. In kernel mode the PAL stops the executive instead, as a
 * bugcheck, and returns 0.
 */
static int exception(seL4_Word off, seL4_Word pc, const seL4_Word *param, unsigned n)
{
	if (!cur->mode)
		return 0;
	read_regs();
	regs.pc = pc;
	if (!vector(&regs, off, ipl, 0, cur->mode, param, n)) {
		print("%%PAL-F-NOVEC, no handler or kernel stack for exception 0x%lx\n", off);
		return 0;
	}
	write_regs(0);
	return 1;
}

/*
 * CHMx, x the mode, CHMK's 0 to CHMU's 3: delivers through the SCB's vector
 * for x, at the same IPL, in mode x or the current mode if that is an inner
 * one, with the code pushed below the frame, as the VAX's CHMx pushes it, so
 * that every register reaches the handler. Without a vector or a stack for
 * that mode, it is a reserved instruction.
 */
static int chmx(int x, seL4_Word code)
{
	seL4_Word pc = cur->mr[seL4_UnknownSyscall_FaultIP];
	int to = x < cur->mode ? x : cur->mode;
	read_regs();
	regs.pc = pc + 4;
	if (!vector(&regs, SCB_CHMK + 4 * x, ipl, to, cur->mode, &code, 1)) {
		if (exception(SCB_OPCDEC, pc, 0, 0))
			return 1;
		print("%%PAL-F-NOVEC, no CHMx handler or stack for mode %u\n", to);
		return 0;
	}
	write_regs(0);
	return 1;
}

/* REI, which in an outer mode is a reserved operand: the executive gets a
 * reserved instruction. */
static int do_rei(void)
{
	read_regs();
	if (!rei(&regs)) {
		if (exception(SCB_OPCDEC, cur->mr[seL4_UnknownSyscall_FaultIP], 0, 0))
			return 1;
		print("%%PAL-F-REI, bad frame at SP 0x%lx, PC 0x%lx\n", (seL4_Word)regs.x18,
		      (seL4_Word)regs.pc);
		return 0;
	}
	if (!deliver(&regs))
		return 0;
	write_regs(0);
	return 1;
}

/*
 * One PAL call or fault from the current context. Those of an outer mode
 * are exceptions the executive handles; those of kernel mode stop it.
 * Returns 0 when the PAL stops serving: the executive halted or took a
 * fault in kernel mode.
 */
static int serve_one(seL4_MessageInfo_t msg)
{
	switch (seL4_MessageInfo_get_label(msg)) {
	case seL4_Fault_UnknownSyscall:
		break;
	case seL4_Fault_VMFault: {
		/* The VAX's access violation parameters: a mask, bit 2 for a
		 * write, then the address. ESR's WnR, bit 6, is a data abort's. */
		seL4_Word addr = seL4_GetMR(seL4_VMFault_Addr), pc = seL4_GetMR(seL4_VMFault_IP);
		seL4_Word write = !seL4_GetMR(seL4_VMFault_PrefetchFault) &&
				  seL4_GetMR(seL4_VMFault_FSR) >> 6 & 1;
		seL4_Word param[2] = { write << 2, addr };
		if (exception(SCB_ACCVIO, pc, param, 2))
			return 1;
		print("%%PAL-F-ACCVIO, access violation at 0x%lx, PC 0x%lx, HWPCB 0x%lx\n", addr,
		      pc, cur->hwpcb);
		return 0;
	}
	case seL4_Fault_UserException: {
		seL4_Word pc = seL4_GetMR(seL4_UserException_FaultIP);
		if (exception(SCB_OPCDEC, pc, 0, 0))
			return 1;
		print("%%PAL-F-OPCDEC, reserved instruction at PC 0x%lx, HWPCB 0x%lx\n", pc,
		      cur->hwpcb);
		return 0;
	}
	default:
		print("%%PAL-F-FAULT, fault %lu, HWPCB 0x%lx\n", seL4_MessageInfo_get_label(msg),
		      cur->hwpcb);
		return 0;
	}
	seL4_Word *mr = cur->mr;
	for (unsigned i = 0; i <= seL4_UnknownSyscall_FaultIP; i++)
		mr[i] = seL4_GetMR(i);
	/* The function in x7's bits 15:0; CHMx's code in 31:16 (DESIGN-0004). */
	seL4_Word pc = mr[seL4_UnknownSyscall_FaultIP], x7 = mr[seL4_UnknownSyscall_X7];
	seL4_Word code = x7 >> 32 ? x7 : x7 & 0xffff;
	seL4_Word a0 = mr[seL4_UnknownSyscall_X0], a1 = mr[seL4_UnknownSyscall_X1];
	seL4_Word v0 = a0;
	/* Privileged calls, 0x00-0x7F, are reserved instructions outside
	 * kernel mode. */
	if (code < 0x80 && cur->mode)
		return exception(SCB_OPCDEC, pc, 0, 0);
	if (x7 >> 16 && (code < CHME || code > CHMU))
		return exception(SCB_OPCDEC, pc, 0, 0);
	switch (code) {
	case HALT:
		print("%%PAL-I-HALT, halted at PC 0x%lx, R0 %lu\n", pc, a0 & 0xffffffff);
		halted = 1;
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
			pending |= 1UL << (a0 & 15);
		return ret(v0);
	case MFPR_SISR:
		return ret(pending & 0xfffe);
	case WTINT:
		/* Returns once an interrupt IPL lets through has been delivered:
		 * now, or at a tick. */
		if (deliverable())
			return ret(0);
		idle = 1;
		return 1;
	case MTPR_TXDB:
		uart_putc(a0);
		return ret(v0);
	case MTPR_DOORBELL:
		if (a0 == 0 && doorbell)
			seL4_Signal(doorbell);
		return ret(v0);
	case MTPR_RXCS:
		rxcs = a0 & RXCS_IE;
		if (rxcs && uart_rx_ready())
			pending |= 1UL << IPL_CONSOLE;
		return ret(v0);
	case MFPR_RXCS:
		return ret(rxcs | (uart_rx_ready() ? RXCS_DONE : 0));
	case MFPR_RXDB:
		return ret(uart_rx_ready() ? uart[UART_DR / 4] & 0xff : 0);
	case READLBLK:
	case WRITELBLK:
		return ret(lblk(a0 & 0xffffffff, a1 & 0xffff, mr[seL4_UnknownSyscall_X2] & 0xffffffff,
				mr[seL4_UnknownSyscall_X3] & 0xffffffff, code == WRITELBLK));
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
			for (int m = 0; m < 4; m++)
				if (c->tcb[m])
					seL4_TCB_Suspend(c->tcb[m]);
			free_space(c);
			c->hwpcb = 0;
		}
		return ret(v0);
	}
	case CHME ... CHMU: {
		static const int mode[] = { 1, 0, 2, 3 }; /* CHME, CHMK, CHMS, CHMU */
		return chmx(mode[code - CHME], x7 >> 16);
	}
	case PROBER:
	case PROBEW:
		return ret(probe(a0 & 0xffffffff, a1 & 0xffff, mr[seL4_UnknownSyscall_X2] & 3,
				 code == PROBEW));
	case RD_PS:
		return ret(psl(seL4_GetMR(seL4_UnknownSyscall_SPSR)));
	case REI:
		return do_rei();
	}
	if (exception(SCB_OPCDEC, pc, 0, 0))
		return 1;
	print("%%PAL-F-OPCDEC, reserved PAL call 0x%lx at PC 0x%lx, R0 0x%lx, R1 0x%lx\n", code,
	      pc, a0, a1);
	return 0;
}

/*
 * A tick of the clock, or the port's completion, or both, by the bits of
 * badge: requests the interval timer interrupt, and the console's if a
 * character waits and RXCS asks for it, or the port's, and, if IPL lets
 * them through, delivers them. The current context is running, since the
 * PAL outranks it and serves each of its PAL calls at once, or waits in
 * WTINT. A running one is stopped wherever it is, and resumes at the
 * handler.
 */
static int tick(seL4_Word badge)
{
	if (badge & PORT_BADGE)
		pending |= 1UL << IPL_PORT;
	if (badge & CLOCK_BADGE)
		pending |= 1UL << IPL_HWCLK;
	if (rxcs && uart_rx_ready())
		pending |= 1UL << IPL_CONSOLE;
	if (!deliverable())
		return 1;
	if (idle) {
		idle = 0;
		return ret(0);
	}
	seL4_TCB_Suspend(cur->tcb[cur->mode]);
	read_regs();
	if (!deliver(&regs))
		return 0;
	write_regs(1);
	return 1;
}

/* Serves the clock's ticks and the current context's PAL calls and faults;
 * the other contexts wait in a PAL call, SWPCTX, or haven't started. */
static void serve(void)
{
	for (;;) {
		seL4_Word badge;
		seL4_MessageInfo_t msg = seL4_Recv(fault_ep, &badge,
						   idle ? idle_reply : cur->reply);
		if (badge & (CLOCK_BADGE | PORT_BADGE)) {
			if (!tick(badge))
				return;
			continue;
		}
		if (badge == TCPIP_BADGE) {
			print("%%PAL-F-TCPIP, the TCP/IP component faulted (%lu), PC 0x%lx\n",
			      seL4_MessageInfo_get_label(msg), seL4_GetMR(seL4_VMFault_IP));
			return;
		}
		if (badge != ctx_badge(cur, cur->mode)) {
			print("%%PAL-F-FAULT, a call from context 0x%lx, not the current one\n", badge);
			return;
		}
		if (!serve_one(msg))
			return;
	}
}

/*
 * The interval timer: a thread of the PAL's that signals the PAL every
 * TICK_US. Its scheduling context has a budget smaller than its period, so
 * seL4_Yield, which gives up the rest of the budget, sleeps until the next
 * period. It outranks the executive's threads, so a tick comes on time.
 * ponytail: the PAL's notification and seL4_Yield, no IPC buffer or TLS;
 * a timer device driver when the PAL has one.
 */
static seL4_CPtr clock_cap, pal_ntfn;
static uint64_t clock_stack[256] __attribute__((aligned(16)));

static void clock_thread(void)
{
	for (;;) {
		seL4_Yield();
		seL4_Signal(clock_cap);
		if (tcpip_tick)
			seL4_Signal(tcpip_tick);
	}
}

static void start_clock(void)
{
	seL4_CPtr ntfn = pal_ntfn = alloc(seL4_NotificationObject, 0);
	seL4_CPtr tcb = alloc(seL4_TCBObject, 0), sc = alloc(seL4_SchedContextObject,
								seL4_MinSchedContextBits);
	clock_cap = mint(ntfn, CLOCK_BADGE);
	idle_reply = alloc(seL4_ReplyObject, 0);
	seL4_Error err = seL4_TCB_BindNotification(seL4_CapInitThreadTCB, ntfn);
	if (!err)
		err = seL4_TCB_Configure(tcb, seL4_CapInitThreadCNode, 0, seL4_CapInitThreadVSpace,
					 0, 0, seL4_CapNull);
	if (!err)
		err = seL4_SchedControl_Configure(sched_control, sc, TICK_US / 10, TICK_US, 0, 0);
	if (!err)
		err = seL4_TCB_SetSchedParams(tcb, seL4_CapInitThreadTCB, seL4_MaxPrio,
					      seL4_MaxPrio, sc, seL4_CapNull);
	seL4_UserContext r = { .pc = (seL4_Word)clock_thread,
			       .sp = (seL4_Word)(clock_stack + 256) };
	if (!err)
		err = seL4_TCB_WriteRegisters(tcb, 1, 0, sizeof r / sizeof(seL4_Word), &r);
	if (err)
		die("starting the clock failed", err);
}

/*
 * Starts [SYSEXE]EXEC.EXE from the system disk in a context of its own
 * whose HWPCB is the RPB's: kernel mode, IPL 31, R11 pointing at the RPB.
 */
static uint8_t exe_file[256 * 1024];
static seL4_Word port_pfn; /* the port's first page */

static void start_exec(void)
{
	exec_vspace = alloc(seL4_ARM_VSpaceObject, 0);
	seL4_Error err = seL4_ARM_ASIDPool_Assign(seL4_CapInitThreadASIDPool, exec_vspace);
	if (err)
		die("seL4_ARM_ASIDPool_Assign failed", err);
	seL4_Word exe_size = f11_boot_file(exe_file, sizeof exe_file);
	if (!exe_size)
		die("no [SYSEXE]EXEC.EXE on the system disk", sizeof exe_file);
	seL4_Word entry = load_image(exe_file, exe_size);
	boot_pages(EXEC_STACK_TOP - EXEC_STACK_PAGES * PAGE_SIZE, 0, EXEC_STACK_PAGES * PAGE_SIZE,
		   PRT_KW << 27);
	boot_pages(RPB_VA, 0, PAGE_SIZE, PRT_KW << 27);
	if (net_slot >= 0) {
		port_pfn = boot_pfn;
		boot_pages(PORT_VA, 0, PORT_PAGES * PAGE_SIZE, PRT_KW << 27);
	}
	uint8_t *rpb = (uint8_t *)quad(RPB_VA);
	uint32_t fields[] = { [RPB_BASE / 4] = RPB_VA, [RPB_PFNCNT / 4] = PFN_COUNT,
			      [RPB_FREEPFN / 4] = boot_pfn, [RPB_BOOTTIME / 4] = boot_time,
			      [RPB_PORT / 4] = net_slot >= 0 ? PORT_VA : 0 };
	memcpy(rpb, fields, sizeof fields);

	cur = new_ctx(RPB_VA + RPB_HWPCB);
	cur->started = 1;
	regs = (seL4_UserContext){ .pc = entry, .sp = EXEC_STACK_TOP, .x18 = EXEC_STACK_TOP,
				   .x28 = RPB_VA }; /* R11 */
	write_regs(1);
	print("EXEC.EXE: started at 0x%lx, %lu of %u pages in use\n", entry, boot_pfn, PFN_COUNT);
}

/*
 * The TCP/IP component (docs/prd/0002-networking.md): tcpip.elf, which
 * build.rs embeds in the root task, started as a thread in an address
 * space and a CSpace of its own (tcpip/include/component.h) if QEMU has a
 * virtio-net device. Its image, data and DMA buffers are one 2 MB large
 * page, so their physical addresses follow from the page's. It shares the
 * port's pages with the executive, drives the device's transport itself
 * and takes its interrupt. The PAL's clock thread signals it each tick,
 * and the executive's doorbell, MTPR_DOORBELL, signals it too. It
 * signals the PAL's notification with PORT_BADGE, which the PAL delivers
 * to the executive as the port's interrupt.
 * ponytail: the 2 MB page is mapped writable and executable; a frame per
 * segment, with its own rights, if the component ever runs code it
 * doesn't trust.
 */
extern const uint8_t tcpip_elf[], tcpip_elf_end[];
#define TCPIP_PAL_VA 0x200000000UL /* where the PAL sees the large page */

static void start_tcpip(void)
{
	if (net_slot < 0)
		return;
	const uint8_t *elf = tcpip_elf;
	seL4_Word size = tcpip_elf_end - tcpip_elf;
	if (size < 64 || u32(elf) != 0x464c457f)
		die("tcpip.elf is not an ELF file", size);
	seL4_CPtr page = alloc(seL4_ARM_LargePageObject, 0);
	map(page, seL4_CapInitThreadVSpace, TCPIP_PAL_VA, seL4_ReadWrite,
	    seL4_ARM_Default_VMAttributes | seL4_ARM_ExecuteNever);
	uint64_t phoff = u64(elf + 32);
	unsigned phnum = u16(elf + 56), phentsize = u16(elf + 54);
	for (unsigned i = 0; i < phnum; i++) {
		const uint8_t *ph = elf + phoff + i * phentsize;
		uint64_t off = u64(ph + 8), va = u64(ph + 16), filesz = u64(ph + 32),
			 memsz = u64(ph + 40);
		if (u32(ph) != 1) /* PT_LOAD */
			continue;
		if (va < TCPIP_BASE || va + memsz > TCPIP_BASE + (1UL << seL4_LargePageBits) ||
		    off + filesz > size)
			die("tcpip.elf: a segment outside its page", va);
		memcpy((uint8_t *)(TCPIP_PAL_VA + va - TCPIP_BASE), elf + off, filesz);
	}
	seL4_ARM_Page_GetAddress_t pa = seL4_ARM_Page_GetAddress(page);
	unmap(page);

	seL4_CPtr vspace = alloc(seL4_ARM_VSpaceObject, 0);
	seL4_Error err = seL4_ARM_ASIDPool_Assign(seL4_CapInitThreadASIDPool, vspace);
	if (err)
		die("seL4_ARM_ASIDPool_Assign failed", err);
	map(page, vspace, TCPIP_BASE, seL4_ReadWrite, seL4_ARM_Default_VMAttributes);
	err = seL4_ARM_Page_Unify_Instruction(page, 0, 1UL << seL4_LargePageBits);
	if (err)
		die("seL4_ARM_Page_Unify_Instruction failed", err);
	for (seL4_Word i = 0; i < PORT_PAGES; i++)
		map(mint(pfn_frame[port_pfn + i], 0), vspace, TCPIP_PORT_VA + i * PAGE_SIZE,
		    seL4_ReadWrite, seL4_ARM_Default_VMAttributes | seL4_ARM_ExecuteNever);
	seL4_Word transport = net_slot * 0x200;
	map(mint(vio_frame[transport / PAGE_SIZE], 0), vspace, TCPIP_MMIO_VA, seL4_ReadWrite,
	    seL4_ARM_ExecuteNever);
	map(mint(uart_frame, 0), vspace, TCPIP_UART_VA, seL4_ReadWrite, seL4_ARM_ExecuteNever);
	seL4_CPtr ipcbuf = alloc(seL4_ARM_SmallPageObject, 0);
	map(ipcbuf, vspace, TCPIP_IPCBUF_VA, seL4_ReadWrite,
	    seL4_ARM_Default_VMAttributes | seL4_ARM_ExecuteNever);

	/* Its CSpace: its notification, the PAL's, and its IRQ's handler. QEMU
	 * virt's virtio-mmio transport n interrupts on SPI 16 + n. */
	seL4_CPtr cnode = alloc(seL4_CapTableObject, TCPIP_CNODE_BITS);
	seL4_CPtr ntfn = alloc(seL4_NotificationObject, 0);
	seL4_CPtr irq = next_slot++;
	err = seL4_IRQControl_Get(seL4_CapIRQControl, 32 + 16 + net_slot, seL4_CapInitThreadCNode,
				  irq, seL4_WordBits);
	if (!err)
		err = seL4_IRQHandler_SetNotification(irq, mint(ntfn, TCPIP_IRQ));
	struct {
		seL4_CPtr cap;
		seL4_Word badge;
	} caps[] = { [TCPIP_CAP_NTFN] = { ntfn, 0 }, [TCPIP_CAP_EXEC] = { pal_ntfn, PORT_BADGE },
		     [TCPIP_CAP_IRQ] = { irq, 0 } };
	for (unsigned i = 1; !err && i < sizeof caps / sizeof caps[0]; i++)
		err = seL4_CNode_Mint(cnode, i, TCPIP_CNODE_BITS, seL4_CapInitThreadCNode,
				      caps[i].cap, seL4_WordBits, seL4_AllRights, caps[i].badge);
	if (err)
		die("the TCP/IP component's capabilities", err);
	tcpip_tick = mint(ntfn, TCPIP_TICK);
	doorbell = mint(ntfn, TCPIP_DOORBELL);

	/* It outranks the executive, which waits for it, and has half of
	 * each tick's time at most. */
	seL4_CPtr tcb = alloc(seL4_TCBObject, 0);
	seL4_CPtr sc = alloc(seL4_SchedContextObject, seL4_MinSchedContextBits);
	err = seL4_TCB_Configure(tcb, cnode,
				 seL4_CNode_CapData_new(0, seL4_WordBits - TCPIP_CNODE_BITS).words[0],
				 vspace, 0, TCPIP_IPCBUF_VA, ipcbuf);
	if (!err)
		err = seL4_SchedControl_Configure(sched_control, sc, TICK_US / 2, TICK_US, 0, 0);
	if (!err)
		err = seL4_TCB_SetSchedParams(tcb, seL4_CapInitThreadTCB, seL4_MaxPrio - 1,
					      seL4_MaxPrio - 1, sc, mint(fault_ep, TCPIP_BADGE));
	seL4_UserContext r = { .pc = u64(elf + 24), .x0 = pa.paddr, .x1 = transport % PAGE_SIZE };
	if (!err)
		err = seL4_TCB_WriteRegisters(tcb, 1, 0, sizeof r / sizeof(seL4_Word), &r);
	if (err)
		die("starting the TCP/IP component failed", err);
}

int main(seL4_BootInfo *bi)
{
	seL4_SetIPCBuffer(bi->ipcBuffer);

	/* The largest RAM untyped; empty slots follow the boot info's caps. */
	seL4_Word n = bi->untyped.end - bi->untyped.start;
	seL4_Word best = n;
	for (seL4_Word i = 0; i < n; i++)
		if (!bi->untypedList[i].isDevice &&
		    (best == n || bi->untypedList[i].sizeBits > bi->untypedList[best].sizeBits))
			best = i;
	if (best == n)
		die("no RAM untyped", n);
	untyped = bi->untyped.start + best;
	next_slot = bi->empty.start;
	uart_init(bi);
	print("hello from the root task\n");

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

	fault_ep = alloc(seL4_EndpointObject, 0);
	start_clock();
	disk_init(bi);
	start_exec();
	start_tcpip();
	serve();

	print("root task done, %lu of %u slots used\n", next_slot, 1u << CONFIG_ROOT_CNODE_SIZE_BITS);
	/* run-qemu.sh's serial-filter.py powers QEMU off on this line (ADR-0008):
	 * EL0 can't make the PSCI call, HVC. */
	if (halted)
		print("%%PAL-I-POWEROFF\n");
	err = seL4_TCB_Suspend(seL4_CapInitThreadTCB);
	print("seL4_TCB_Suspend failed: %u\n", err);
	return 0;
}
