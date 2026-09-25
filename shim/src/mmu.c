/* Page tables, caches and the switch from Limine's MMU setup to seL4's. */
#include "shim.h"

#define PTE_TABLE   3UL
#define PTE_BLOCK   1UL
#define PTE_AF      (1UL << 10)
#define PTE_ATTR(i) ((uint64_t)(i) << 2)
#define PTE_XN      (3UL << 53) /* PXN | UXN */
#define PTE_ADDR    0x0000fffffffff000UL
#define BLOCK_2M    (1UL << 21)

/*
 * While Limine's MAIR_EL1 is live: Attr0 is Normal WB and Attr1 the
 * framebuffer type, and base revision 4+ guarantees the other slots unused,
 * so the shim claims Attr2 for Device-nGnRnE.
 */
#define LIMINE_ATTR_NORMAL 0
#define LIMINE_ATTR_DEVICE 2

/* The MAIR_EL1 and TCR_EL1 seL4 expects; see shim/README.md. */
#define SEL4_MAIR        0x0000aaff440c0400UL
#define SEL4_ATTR_NORMAL 4
#define SEL4_TCR                                                  \
	(16UL | 16UL << 16 |                     /* T0SZ, T1SZ */ \
	 1UL << 8 | 1UL << 10 | 1UL << 24 | 1UL << 26 | /* WBWA walks */ \
	 2UL << 30 |                             /* TG1 4 KiB */  \
	 1UL << 36)                              /* 16-bit ASIDs */

/* What tramp needs, read before it turns the MMU off. */
struct handoff {
	uint64_t regs[6], entry, ttbr0, ttbr1, mair, tcr;
};
_Static_assert(offsetof(struct handoff, tcr) == 80, "tramp in entry.S hard-codes the layout");

extern char tramp[]; /* entry.S, page aligned and shorter than a page */

static uint64_t tables[8][512] __attribute__((aligned(4096)));
static unsigned ntables;
static uint64_t pv_offset; /* shim physical minus virtual address */

static uint64_t pa(const void *va)
{
	return (uint64_t)va + pv_offset;
}

static uint64_t *new_table(void)
{
	if (ntables == sizeof(tables) / sizeof(tables[0]))
		panic("out of page tables");
	return tables[ntables++];
}

/* Map the 1 << shift bytes block (30: 1 GiB, 21: 2 MiB) containing vaddr. */
static void map_block(uint64_t *root, uint64_t vaddr, uint64_t paddr, unsigned shift,
		      uint64_t attrs)
{
	uint64_t *t = root;
	for (unsigned s = 39; s > shift; s -= 9) {
		uint64_t *e = &t[(vaddr >> s) & 511];
		if (!*e)
			*e = pa(new_table()) | PTE_TABLE;
		t = (uint64_t *)((*e & PTE_ADDR) - pv_offset);
	}
	t[(vaddr >> shift) & 511] = (paddr & ~((1UL << shift) - 1)) | attrs | PTE_AF | PTE_BLOCK;
}

/*
 * Limine leaves TTBR0_EL1 to us (base revision 1+) while its higher-half
 * tables stay live in TTBR1_EL1. Identity-map the 1 GiB around tramp and the
 * 1 GiB around the UART there, so the shim can print and later jump to tramp
 * at its physical address.
 */
void mmu_init(uint64_t shim_vbase, uint64_t shim_pbase, uint64_t uart_pa)
{
	pv_offset = shim_pbase - shim_vbase;

	uint64_t mair;
	__asm__ volatile("mrs %0, mair_el1" : "=r"(mair));
	mair &= ~(0xffUL << (8 * LIMINE_ATTR_DEVICE)); /* 0x00: Device-nGnRnE */
	__asm__ volatile("msr mair_el1, %0; isb" ::"r"(mair));

	uint64_t *ttbr0 = new_table();
	map_block(ttbr0, pa(tramp), pa(tramp), 30, PTE_ATTR(LIMINE_ATTR_NORMAL));
	/* If both share a GiB the UART wins and tramp runs from Device memory. */
	map_block(ttbr0, uart_pa, uart_pa, 30, PTE_ATTR(LIMINE_ATTR_DEVICE) | PTE_XN);
	__asm__ volatile("dsb ishst; msr ttbr0_el1, %0; isb; tlbi vmalle1; dsb nsh; isb"
			 ::"r"(pa(ttbr0)) : "memory");
}

/* Clean and invalidate [va, va + len) to the point of coherency. */
void dcache_clean(uint64_t va, uint64_t len)
{
	uint64_t ctr;
	__asm__ volatile("mrs %0, ctr_el0" : "=r"(ctr));
	uint64_t line = 4UL << (ctr >> 16 & 15); /* CTR_EL0.DminLine, in words */
	for (uint64_t p = va & ~(line - 1); p < va + len; p += line)
		__asm__ volatile("dc civac, %0" ::"r"(p) : "memory");
	__asm__ volatile("dsb sy" ::: "memory");
}

/*
 * Build the tables the elfloader would: TTBR1 maps the kernel with 2 MiB
 * blocks from its first vaddr to the end of that GiB, TTBR0 identity-maps
 * tramp. Then jump to tramp at its physical address; it switches to these
 * tables with seL4's MAIR and TCR and enters the kernel with regs in x0-x5.
 */
void mmu_enter_kernel(const struct elf *kernel, const uint64_t regs[6])
{
	if ((kernel->vbase | kernel->pbase) & (BLOCK_2M - 1))
		panic("kernel base 0x%lx (phys 0x%lx) is not 2 MiB aligned", kernel->vbase,
		      kernel->pbase);
	if (kernel->vbase >> 30 != (kernel->vend - 1) >> 30)
		panic("kernel 0x%lx-0x%lx crosses a 1 GiB boundary", kernel->vbase, kernel->vend);

	uint64_t *ttbr1 = new_table(), *ttbr0 = new_table();
	for (uint64_t v = kernel->vbase, p = kernel->pbase; v >> 30 == kernel->vbase >> 30;
	     v += BLOCK_2M, p += BLOCK_2M)
		map_block(ttbr1, v, p, 21, PTE_ATTR(SEL4_ATTR_NORMAL));
	map_block(ttbr0, pa(tramp), pa(tramp), 21, PTE_ATTR(SEL4_ATTR_NORMAL));

	uint64_t mmfr0;
	__asm__ volatile("mrs %0, id_aa64mmfr0_el1" : "=r"(mmfr0));
	struct handoff h = {
		.regs = { regs[0], regs[1], regs[2], regs[3], regs[4], regs[5] },
		.entry = kernel->entry,
		.ttbr0 = pa(ttbr0),
		.ttbr1 = pa(ttbr1),
		.mair = SEL4_MAIR,
		.tcr = SEL4_TCR | (mmfr0 & 7) << 32, /* IPS = PARange */
	};
	dcache_clean((uint64_t)tables, sizeof(tables));
	dcache_clean((uint64_t)tramp, PAGE_SIZE);
	((void (*)(const struct handoff *))pa(tramp))(&h);
	__builtin_unreachable();
}
