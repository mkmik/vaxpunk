# kernel/: standalone seL4 build through seL4's own CMake. make -C kernel
# Rebuilds only when config.cmake, qemu.env, this Makefile or the checked-out
# seL4 commit change.
include qemu.env

CROSS_COMPILE ?= $(if $(shell command -v aarch64-elf-gcc),aarch64-elf-,aarch64-linux-gnu-)
VENV := .venv

$(if $(wildcard seL4/CMakeLists.txt),,$(error kernel/seL4 is missing, run: git submodule update --init))

# Record the seL4 commit in a file that changes only when the commit does.
SEL4_REV := $(shell git -C seL4 rev-parse HEAD)
$(shell mkdir -p build; echo $(SEL4_REV) | cmp -s - build/sel4.rev || echo $(SEL4_REV) > build/sel4.rev)

out/kernel.elf: config.cmake qemu.env build/sel4.rev Makefile | $(VENV)/ok
	rm -rf build/cmake build/install out
	PATH="$(CURDIR)/$(VENV)/bin:$$PATH" cmake -G Ninja -S seL4 -B build/cmake -C config.cmake \
		-DCMAKE_TOOLCHAIN_FILE=$(CURDIR)/seL4/gcc.cmake -DCROSS_COMPILER_PREFIX=$(CROSS_COMPILE) \
		-DCMAKE_INSTALL_PREFIX=$(CURDIR)/build/install \
		-DARM_CPU=$(QEMU_CPU) -DQEMU_MEMORY=$(QEMU_MEM) -DQEMU_GIC_VERSION=$(QEMU_GIC)
	PATH="$(CURDIR)/$(VENV)/bin:$$PATH" ninja -C build/cmake install
	mkdir -p out
	cp -R build/install/libsel4/include out/include
	cp build/install/support/kernel.dtb build/install/support/platform_gen.json out/
	cp build/install/bin/kernel.elf out/kernel.elf

$(VENV)/ok: requirements.txt
	uv venv --allow-existing $(VENV)
	uv pip install --python $(VENV) -r requirements.txt
	touch $@

clean:
	rm -rf build out

.PHONY: clean
