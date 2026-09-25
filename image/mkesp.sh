#!/bin/sh
# Build out/esp.img: a 64 MiB FAT32 ESP holding Limine, its config, the shim,
# the seL4 kernel and the root task. mtools only: no root, no mounting.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
img=$root/out/esp.img

mkdir -p "$root/out"
rm -f "$img"
mformat -i "$img" -C -T 131072 -h 64 -s 32 -F -v ESP ::
mmd -i "$img" ::/EFI ::/EFI/BOOT ::/boot
mcopy -i "$img" "$root/third_party/limine/BOOTAA64.EFI" ::/EFI/BOOT/
mcopy -i "$img" "$root/image/limine.conf" "$root/shim/out/shim.elf" \
	"$root/kernel/out/kernel.elf" "$root/roottask/out/roottask.elf" ::/boot/
