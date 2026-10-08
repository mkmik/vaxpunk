#!/bin/sh
# Build out/esp.img: a 64 MiB FAT32 ESP holding Limine, its config and the
# shim, seL4 kernel and root task in out/, where cargo run -p boot puts them.
# mtools only: no root, no mounting.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
img=$root/out/esp.img

[ -f "$root/third_party/limine/BOOTAA64.EFI" ] || "$root/scripts/fetch-limine.sh"
rm -f "$img"
mformat -i "$img" -C -T 131072 -h 64 -s 32 -F -v ESP ::
mmd -i "$img" ::/EFI ::/EFI/BOOT ::/boot
mcopy -i "$img" "$root/third_party/limine/BOOTAA64.EFI" ::/EFI/BOOT/
mcopy -i "$img" "$root/boot/limine.conf" "$root/out/shim.elf" \
	"$root/out/kernel.elf" "$root/out/roottask.elf" ::/boot/
