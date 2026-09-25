/* Page tables and MAIR/TTBR handling. */
#include "shim.h"

#define PTE_TABLE   3UL
#define PTE_BLOCK   1UL
#define PTE_AF      (1UL << 10)
#define PTE_ATTR(i) ((uint64_t)(i) << 2)
#define PTE_XN      (3UL << 53) /* PXN | UXN */
#define PTE_ADDR    0x0000fffffffff000UL

/*
 * MAIR_EL1 slots while Limine's MAIR is live. Limine base revision 4+ uses
 * Attr0 (Normal WB) and Attr1 (framebuffer) and guarantees the rest unused,
 * so the shim claims Attr2 for Device-nGnRnE.
 */
#define LIMINE_ATTR_DEVICE 2

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

void mmu_init(uint64_t shim_vbase, uint64_t shim_pbase)
{
	pv_offset = shim_pbase - shim_vbase;
}

/*
 * Limine leaves TTBR0_EL1 to us (base revision 1+). Point it at an identity
 * map of the 1 GiB around the UART so the shim can print while Limine's
 * higher-half tables stay live in TTBR1_EL1.
 */
void mmu_map_uart(uint64_t uart_pa)
{
	uint64_t mair;
	__asm__ volatile("mrs %0, mair_el1" : "=r"(mair));
	mair &= ~(0xffUL << (8 * LIMINE_ATTR_DEVICE)); /* 0x00: Device-nGnRnE */
	__asm__ volatile("msr mair_el1, %0; isb" ::"r"(mair));

	uint64_t *ttbr0 = new_table();
	map_block(ttbr0, uart_pa, uart_pa, 30, PTE_ATTR(LIMINE_ATTR_DEVICE) | PTE_XN);
	__asm__ volatile("dsb ishst; msr ttbr0_el1, %0; isb; tlbi vmalle1; dsb nsh; isb"
			 ::"r"(pa(ttbr0)) : "memory");
}
