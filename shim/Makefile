# shim/: freestanding Limine-protocol ELF. Builds on its own: make -C shim
CROSS_COMPILE ?= $(if $(shell command -v aarch64-elf-gcc),aarch64-elf-,aarch64-linux-gnu-)
CC := $(CROSS_COMPILE)gcc
SEL4_PLATFORM ?= ../kernel/out/platform_gen.json

CFLAGS := -O2 -g -Wall -Wextra -ffreestanding -mgeneral-regs-only -fno-pie \
	-fno-stack-protector -fno-asynchronous-unwind-tables \
	-fno-tree-loop-distribute-patterns -I. -Iinclude -Iout
LDFLAGS := -nostdlib -static -no-pie -T linker.ld -Wl,--build-id=none \
	-Wl,-z,max-page-size=4096

SRCS := $(wildcard src/*.c src/*.S)

out/shim.elf: $(SRCS) $(wildcard include/*.h) limine.h linker.ld Makefile out/sel4_ram.h
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $(SRCS)

# seL4's RAM ranges as C initializers, from the platform the kernel was built for.
out/sel4_ram.h: $(SEL4_PLATFORM)
	@mkdir -p out
	python3 -c 'import json, sys; [print("{ 0x%x, 0x%x }," % (r["start"], r["end"])) for r in json.load(sys.stdin)["memory"]]' < $< > $@

$(SEL4_PLATFORM):
	$(error $@ is missing, build the kernel first: make -C kernel)

clean:
	rm -rf out

.PHONY: clean
