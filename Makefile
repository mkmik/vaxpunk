# Convenience only: each component builds on its own with make -C <dir>.
all: out/esp.img

kernel:
	$(MAKE) -C kernel

shim roottask: kernel
	$(MAKE) -C $@

third_party/limine/BOOTAA64.EFI:
	scripts/fetch-limine.sh

out/esp.img: shim roottask third_party/limine/BOOTAA64.EFI
	image/mkesp.sh

run: out/esp.img
	scripts/run-qemu.sh

clean:
	for d in kernel shim roottask; do $(MAKE) -C $$d clean; done
	rm -rf out

.PHONY: all kernel shim roottask out/esp.img run clean
