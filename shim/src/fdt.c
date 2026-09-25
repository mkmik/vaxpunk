/* Just enough of a flattened device tree walker to find the PL011 UART. */
#include "shim.h"

#define FDT_MAGIC      0xd00dfeed
#define FDT_BEGIN_NODE 1
#define FDT_END_NODE   2
#define FDT_PROP       3
#define FDT_NOP        4

static uint32_t be32(const void *p)
{
	const uint8_t *b = p;
	return (uint32_t)b[0] << 24 | (uint32_t)b[1] << 16 | (uint32_t)b[2] << 8 | b[3];
}

static size_t slen(const char *s)
{
	size_t n = 0;
	while (s[n])
		n++;
	return n;
}

int streq(const char *a, const char *b)
{
	while (*a && *a == *b)
		a++, b++;
	return *a == *b;
}

/* Returns the blob's total size, or 0 if it is not a device tree. */
uint32_t fdt_size(const void *fdt)
{
	return be32(fdt) == FDT_MAGIC ? be32((const uint8_t *)fdt + 4) : 0;
}

/*
 * Returns the physical address of the first node compatible with
 * "arm,pl011", or 0. Properties of a node precede its children, so a node is
 * complete when the next BEGIN_NODE or END_NODE token shows up.
 */
uint64_t fdt_find_pl011(const void *fdt)
{
	if (!fdt_size(fdt))
		return 0;
	const uint8_t *p = (const uint8_t *)fdt + be32((const uint8_t *)fdt + 8);
	const char *strings = (const char *)fdt + be32((const uint8_t *)fdt + 12);
	uint32_t acells[16] = { 2 }; /* #address-cells by depth, default 2 */
	int depth = 0, match = 0;
	const uint8_t *reg = NULL;

	for (;;) {
		uint32_t tok = be32(p);
		p += 4;
		if (tok == FDT_BEGIN_NODE || tok == FDT_END_NODE) {
			if (match && reg)
				return acells[depth - 1] == 1 ? be32(reg) :
				       (uint64_t)be32(reg) << 32 | be32(reg + 4);
			match = 0;
			reg = NULL;
			if (tok == FDT_END_NODE) {
				depth--;
				continue;
			}
			if (++depth == 16)
				return 0;
			acells[depth] = 2;
			p += (slen((const char *)p) + 4) & ~3UL;
		} else if (tok == FDT_PROP) {
			uint32_t len = be32(p);
			const char *name = strings + be32(p + 4);
			const char *val = (const char *)p + 8;
			if (streq(name, "compatible")) {
				for (const char *s = val; s < val + len; s += slen(s) + 1)
					match |= streq(s, "arm,pl011");
			} else if (streq(name, "reg")) {
				reg = (const uint8_t *)val;
			} else if (streq(name, "#address-cells")) {
				acells[depth] = be32(val);
			}
			p += 8 + ((len + 3) & ~3UL);
		} else if (tok != FDT_NOP) {
			return 0; /* FDT_END or garbage */
		}
	}
}
