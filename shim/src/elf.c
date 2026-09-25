/* ELF64 parsing and loading for the kernel and root task images. */
#include "shim.h"

#define PT_LOAD 1

typedef struct {
	uint8_t e_ident[16];
	uint16_t e_type, e_machine;
	uint32_t e_version;
	uint64_t e_entry, e_phoff, e_shoff;
	uint32_t e_flags;
	uint16_t e_ehsize, e_phentsize, e_phnum, e_shentsize, e_shnum, e_shstrndx;
} Elf64_Ehdr;

typedef struct {
	uint32_t p_type, p_flags;
	uint64_t p_offset, p_vaddr, p_paddr, p_filesz, p_memsz, p_align;
} Elf64_Phdr;

static const Elf64_Phdr *phdrs(const struct elf *e, unsigned *n)
{
	const Elf64_Ehdr *h = (const Elf64_Ehdr *)e->file;
	*n = h->e_phnum;
	return (const Elf64_Phdr *)(e->file + h->e_phoff);
}

/* Validates an in-memory ELF file and records its load bounds. Panics if malformed. */
void elf_parse(struct elf *e, const char *name, const void *file, uint64_t size)
{
	const Elf64_Ehdr *h = file;
	e->name = name;
	e->file = file;
	if (size < sizeof(*h) || h->e_ident[0] != 0x7f || h->e_ident[1] != 'E' ||
	    h->e_ident[2] != 'L' || h->e_ident[3] != 'F')
		panic("%s: not an ELF file", name);
	if (h->e_ident[4] != 2 || h->e_ident[5] != 1 || h->e_machine != 183)
		panic("%s: not a little-endian ELF64 for AArch64", name);
	if (h->e_type != 2)
		panic("%s: not a static executable (ET_EXEC)", name);
	if (h->e_phentsize != sizeof(Elf64_Phdr) || h->e_phoff > size ||
	    (size - h->e_phoff) / sizeof(Elf64_Phdr) < h->e_phnum)
		panic("%s: bad program header table", name);

	uint64_t vmin = UINT64_MAX, vmax = 0, pmin = 0;
	unsigned n;
	const Elf64_Phdr *ph = phdrs(e, &n);
	for (unsigned i = 0; i < n; i++, ph++) {
		if (ph->p_type != PT_LOAD || !ph->p_memsz)
			continue;
		if (ph->p_offset > size || ph->p_filesz > size - ph->p_offset ||
		    ph->p_filesz > ph->p_memsz || ph->p_vaddr + ph->p_memsz < ph->p_vaddr)
			panic("%s: segment %u lies outside the file or wraps", name, i);
		if (vmin != UINT64_MAX && ph->p_vaddr - ph->p_paddr != vmin - pmin)
			panic("%s: segments disagree on the virtual-physical offset", name);
		if (ph->p_vaddr < vmin) {
			vmin = ph->p_vaddr;
			pmin = ph->p_paddr;
		}
		if (ph->p_vaddr + ph->p_memsz > vmax)
			vmax = ph->p_vaddr + ph->p_memsz;
	}
	if (vmin == UINT64_MAX)
		panic("%s: no loadable segments", name);
	if (vmin % PAGE_SIZE || pmin % PAGE_SIZE)
		panic("%s: first segment not page aligned", name);
	e->vbase = vmin;
	e->vend = ALIGN_UP(vmax, PAGE_SIZE);
	e->pbase = pmin;
	e->entry = h->e_entry;
	if (e->entry < e->vbase || e->entry >= e->vend)
		panic("%s: entry point 0x%lx outside the image", name, e->entry);
}

/* Copies the image to dst (a pointer to its physical destination) and zeroes the rest. */
void elf_load(const struct elf *e, uint8_t *dst)
{
	memset(dst, 0, e->vend - e->vbase);
	unsigned n;
	const Elf64_Phdr *ph = phdrs(e, &n);
	for (unsigned i = 0; i < n; i++, ph++)
		if (ph->p_type == PT_LOAD)
			memcpy(dst + (ph->p_vaddr - e->vbase), e->file + ph->p_offset, ph->p_filesz);
}
