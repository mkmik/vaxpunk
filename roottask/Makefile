# roottask/: freestanding C root task. Builds on its own: make -C roottask
CROSS_COMPILE ?= $(if $(shell command -v aarch64-elf-gcc),aarch64-elf-,aarch64-linux-gnu-)
CC := $(CROSS_COMPILE)gcc
SEL4_INCLUDE ?= ../kernel/out/include

CFLAGS := -O2 -g -Wall -Wextra -ffreestanding -fno-pie -fno-stack-protector \
	-fno-asynchronous-unwind-tables -I$(SEL4_INCLUDE)
LDFLAGS := -nostdlib -static -no-pie -T linker.ld -Wl,--build-id=none \
	-Wl,-z,max-page-size=4096

SRCS := $(wildcard src/*.c src/*.S)

out/roottask.elf: $(SRCS) linker.ld Makefile $(SEL4_INCLUDE)/sel4/sel4.h
	@mkdir -p out
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $(SRCS)

$(SEL4_INCLUDE)/sel4/sel4.h:
	$(error $(SEL4_INCLUDE) is missing, build the kernel first: make -C kernel)

clean:
	rm -rf out

.PHONY: clean
