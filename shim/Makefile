# shim/: freestanding Limine-protocol ELF. Builds on its own: make -C shim
CROSS_COMPILE ?= $(if $(shell command -v aarch64-elf-gcc),aarch64-elf-,aarch64-linux-gnu-)
CC := $(CROSS_COMPILE)gcc

CFLAGS := -O2 -g -Wall -Wextra -ffreestanding -mgeneral-regs-only -fno-pie \
	-fno-stack-protector -fno-asynchronous-unwind-tables \
	-fno-tree-loop-distribute-patterns -I. -Iinclude
LDFLAGS := -nostdlib -static -no-pie -T linker.ld -Wl,--build-id=none \
	-Wl,-z,max-page-size=4096

SRCS := $(wildcard src/*.c src/*.S)

out/shim.elf: $(SRCS) $(wildcard include/*.h) limine.h linker.ld Makefile
	@mkdir -p out
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $(SRCS)

clean:
	rm -rf out

.PHONY: clean
