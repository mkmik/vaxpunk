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

static const char *const memmap_names[] = {
	"usable", "reserved", "ACPI reclaimable", "ACPI NVS", "bad",
	"bootloader reclaimable", "executable and modules", "framebuffer", "reserved mapped",
};

void shim_main(void)
{
	/* Nothing can be printed until the UART is found and mapped. */
	if (!LIMINE_BASE_REVISION_SUPPORTED(base_revision) || !exec_req.response || !dtb_req.response)
		halt();
	mmu_init(exec_req.response->virtual_base, exec_req.response->physical_base);
	uint64_t uart_pa = fdt_find_pl011(dtb_req.response->dtb_ptr);
	if (!uart_pa)
		halt();
	mmu_map_uart(uart_pa);
	uart = (volatile uint32_t *)uart_pa;

	uint64_t el;
	__asm__ volatile("mrs %0, CurrentEL" : "=r"(el));
	print("\nvaxpunk shim: EL%lu, shim at 0x%lx, UART at 0x%lx\n", el >> 2 & 3,
	      exec_req.response->physical_base, uart_pa);

	struct limine_memmap_response *mm = memmap_req.response;
	for (uint64_t i = 0; mm && i < mm->entry_count; i++) {
		struct limine_memmap_entry *e = mm->entries[i];
		print("  mem 0x%lx-0x%lx %s\n", e->base, e->base + e->length,
		      e->type < 9 ? memmap_names[e->type] : "?");
	}
	struct limine_module_response *mods = module_req.response;
	for (uint64_t i = 0; mods && i < mods->module_count; i++)
		print("  module '%s' %s at 0x%lx size 0x%lx\n", mods->modules[i]->string,
		      mods->modules[i]->path, (uint64_t)mods->modules[i]->address,
		      mods->modules[i]->size);
	print("hhdm 0x%lx\n", hhdm_req.response ? hhdm_req.response->offset : 0);
	halt();
}
