/* Declarations shared by the shim's source files. */
#pragma once

#include <stddef.h>
#include <stdint.h>

/* main.c */
void *memset(void *dst, int c, size_t n);
void *memcpy(void *dst, const void *src, size_t n);
void print(const char *fmt, ...) __attribute__((format(printf, 1, 2)));
__attribute__((noreturn, format(printf, 1, 2))) void panic(const char *fmt, ...);

/* fdt.c */
uint32_t fdt_size(const void *fdt);
uint64_t fdt_find_pl011(const void *fdt);

/* mmu.c */
void mmu_init(uint64_t shim_vbase, uint64_t shim_pbase);
void mmu_map_uart(uint64_t uart_pa);
