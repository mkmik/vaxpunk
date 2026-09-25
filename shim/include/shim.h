/* Declarations shared by the shim's source files. */
#pragma once

#include <stddef.h>
#include <stdint.h>

#define PAGE_SIZE      0x1000UL
#define ALIGN_UP(x, a) (((x) + (a) - 1) & ~((a) - 1))

/* An ELF image and the virtual range its loadable segments span. */
struct elf {
	const char *name;
	const uint8_t *file;
	uint64_t vbase, vend; /* page aligned */
	uint64_t pbase;       /* physical address of vbase as linked */
	uint64_t entry;
};

/* main.c */
void *memset(void *dst, int c, size_t n);
void *memcpy(void *dst, const void *src, size_t n);
void print(const char *fmt, ...) __attribute__((format(printf, 1, 2)));
__attribute__((noreturn, format(printf, 1, 2))) void panic(const char *fmt, ...);

/* elf.c */
void elf_parse(struct elf *e, const char *name, const void *file, uint64_t size);
void elf_load(const struct elf *e, uint8_t *dst);

/* fdt.c */
int streq(const char *a, const char *b);
uint32_t fdt_size(const void *fdt);
uint64_t fdt_find_pl011(const void *fdt);

/* mmu.c */
void mmu_init(uint64_t shim_vbase, uint64_t shim_pbase);
void mmu_map_uart(uint64_t uart_pa);
