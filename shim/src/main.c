/*
 * shim: a Limine-protocol executable that loads the seL4 kernel and the root
 * task, then enters seL4 in the state the stock elfloader leaves it in.
 * The contract is documented in shim/README.md.
 */
#include <stdarg.h>

#include "limine.h"
#include "shim.h"

#define REQUEST __attribute__((used, section(".limine_requests")))

REQUEST static volatile uint64_t base_revision[] = LIMINE_BASE_REVISION(6);
REQUEST static volatile struct limine_hhdm_request hhdm_req = { .id = LIMINE_HHDM_REQUEST_ID };
REQUEST static volatile struct limine_memmap_request memmap_req = { .id = LIMINE_MEMMAP_REQUEST_ID };
REQUEST static volatile struct limine_module_request module_req = { .id = LIMINE_MODULE_REQUEST_ID };
REQUEST static volatile struct limine_dtb_request dtb_req = { .id = LIMINE_DTB_REQUEST_ID };
REQUEST static volatile struct limine_executable_address_request exec_req = {
	.id = LIMINE_EXECUTABLE_ADDRESS_REQUEST_ID
};

static volatile uint32_t *uart; /* PL011, set once it is mapped */

void *memset(void *dst, int c, size_t n)
{
	uint8_t *d = dst;
	while (n--)
		*d++ = (uint8_t)c;
	return dst;
}

void *memcpy(void *dst, const void *src, size_t n)
{
	uint8_t *d = dst;
	const uint8_t *s = src;
	while (n--)
		*d++ = *s++;
	return dst;
}

static void putc(char c)
{
	if (!uart)
		return;
	if (c == '\n')
		putc('\r');
	while (uart[0x18 / 4] & (1 << 5)) /* FR.TXFF */
		;
	uart[0] = (uint8_t)c;
}

static void vprint(const char *fmt, va_list ap)
{
	for (; *fmt; fmt++) {
		if (*fmt != '%') {
			putc(*fmt);
			continue;
		}
		int wide = *++fmt == 'l';
		fmt += wide;
		if (*fmt == 's') {
			for (const char *s = va_arg(ap, const char *); *s; s++)
				putc(*s);
			continue;
		}
		uint64_t v = wide ? va_arg(ap, uint64_t) : va_arg(ap, uint32_t);
		unsigned base = *fmt == 'x' ? 16 : 10;
		char buf[20];
		int n = 0;
		do {
			buf[n++] = "0123456789abcdef"[v % base];
			v /= base;
		} while (v);
		while (n)
			putc(buf[--n]);
	}
}

void print(const char *fmt, ...)
{
	va_list ap;
	va_start(ap, fmt);
	vprint(fmt, ap);
	va_end(ap);
}

static __attribute__((noreturn)) void halt(void)
{
	for (;;)
		__asm__ volatile("wfe");
}

void panic(const char *fmt, ...)
{
	va_list ap;
	va_start(ap, fmt);
	print("shim: panic: ");
	vprint(fmt, ap);
	va_end(ap);
	print("\n");
	halt();
}

void shim_exception(uint64_t esr, uint64_t elr, uint64_t far)
{
	panic("exception: ESR 0x%lx ELR 0x%lx FAR 0x%lx", esr, elr, far);
}

/* RAM ranges seL4 was built for, generated from kernel/out/platform_gen.json. */
static const struct {
	uint64_t start, end;
} sel4_ram[] = {
#include "sel4_ram.h"
};

static const char *const memmap_names[] = {
	"usable", "reserved", "ACPI reclaimable", "ACPI NVS", "bad",
	"bootloader reclaimable", "executable and modules", "framebuffer", "reserved mapped",
};

static const struct limine_file *find_module(const char *name)
{
	struct limine_module_response *r = module_req.response;
	for (uint64_t i = 0; r && i < r->module_count; i++)
		if (streq(r->modules[i]->string, name))
			return r->modules[i];
	panic("module '%s' not found, check module_string in limine.conf", name);
}

/*
 * A placement must lie in RAM that is free now (Limine "usable") and that
 * seL4 was built to own. The shim, the modules and Limine's own data are
 * never "usable", so this also rules out overlapping any of them.
 */
static void check_placement(const char *what, uint64_t start, uint64_t end)
{
	struct limine_memmap_response *mm = memmap_req.response;
	uint64_t run_start = 0, run_end = 0;
	int usable = 0, owned = 0;
	for (uint64_t i = 0; i < mm->entry_count; i++) {
		struct limine_memmap_entry *e = mm->entries[i];
		if (e->type != LIMINE_MEMMAP_USABLE)
			continue;
		if (e->base != run_end) /* entries are sorted: merge adjacent usable ones */
			run_start = e->base;
		run_end = e->base + e->length;
		usable |= run_start <= start && end <= run_end;
	}
	for (size_t i = 0; i < sizeof(sel4_ram) / sizeof(sel4_ram[0]); i++)
		owned |= sel4_ram[i].start <= start && end <= sel4_ram[i].end;
	if (!usable) {
		for (uint64_t i = 0; i < mm->entry_count; i++)
			print("  0x%lx-0x%lx %s\n", mm->entries[i]->base,
			      mm->entries[i]->base + mm->entries[i]->length,
			      mm->entries[i]->type < 9 ? memmap_names[mm->entries[i]->type] : "?");
		panic("%s 0x%lx-0x%lx overlaps memory that is not free (map above)", what, start, end);
	}
	if (!owned)
		panic("%s 0x%lx-0x%lx is outside the RAM seL4 was built for", what, start, end);
}

void shim_main(void)
{
	/* Nothing can be printed until the UART is found and mapped. */
	if (!LIMINE_BASE_REVISION_SUPPORTED(base_revision) || !exec_req.response || !dtb_req.response)
		halt();
	const void *dtb = dtb_req.response->dtb_ptr;
	uint64_t uart_pa = fdt_find_pl011(dtb);
	if (!uart_pa)
		halt();
	mmu_init(exec_req.response->virtual_base, exec_req.response->physical_base, uart_pa);
	uart = (volatile uint32_t *)uart_pa;

	uint64_t el;
	__asm__ volatile("mrs %0, CurrentEL" : "=r"(el));
	print("\nvaxpunk shim: shim at 0x%lx, UART at 0x%lx\n", exec_req.response->physical_base,
	      uart_pa);
	if ((el >> 2 & 3) != 1)
		panic("entered at EL%lu, the kernel is built for EL1", el >> 2 & 3);
	if (!memmap_req.response || !hhdm_req.response)
		panic("no memory map or HHDM from Limine");

	struct elf kernel, user;
	const struct limine_file *kf = find_module("kernel"), *uf = find_module("roottask");
	elf_parse(&kernel, "kernel", kf->address, kf->size);
	elf_parse(&user, "roottask", uf->address, uf->size);

	uint64_t k_start = kernel.pbase, k_end = k_start + (kernel.vend - kernel.vbase);
	uint64_t dtb_start = ALIGN_UP(k_end, PAGE_SIZE), dtb_end = dtb_start + fdt_size(dtb);
	uint64_t ui_start = ALIGN_UP(dtb_end, PAGE_SIZE), ui_end = ui_start + (user.vend - user.vbase);
	check_placement("kernel", k_start, k_end);
	check_placement("DTB", dtb_start, dtb_end);
	check_placement("root task", ui_start, ui_end);
	print("shim: kernel    0x%lx-0x%lx entry 0x%lx\n", k_start, k_end, kernel.entry);
	print("shim: DTB       0x%lx-0x%lx\n", dtb_start, dtb_end);
	print("shim: root task 0x%lx-0x%lx vaddr 0x%lx entry 0x%lx\n", ui_start, ui_end,
	      user.vbase, user.entry);

	uint8_t *hhdm = (uint8_t *)hhdm_req.response->offset;
	elf_load(&kernel, hhdm + k_start);
	memcpy(hhdm + dtb_start, dtb, dtb_end - dtb_start);
	elf_load(&user, hhdm + ui_start);
	dcache_clean((uint64_t)hhdm + k_start, ui_end - k_start);

	print("shim: entering seL4\n");
	const uint64_t regs[6] = { ui_start, ui_end, ui_start - user.vbase, user.entry,
				   dtb_start, dtb_end - dtb_start };
	mmu_enter_kernel(&kernel, regs);
}
