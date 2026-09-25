#!/bin/sh
# Boot out/esp.img: EDK2 -> Limine -> shim -> seL4 -> root task.
# Usage: run-qemu.sh [--gdb] [--hvf]. Quit with Ctrl-A x.
#   --gdb  wait for a debugger on localhost:1234 (-s -S)
#   --hvf  use Hypervisor.framework instead of TCG (best effort, macOS only)
# EDK2_FW overrides the firmware image.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
. "$root/kernel/qemu.env"

cpu="-cpu $QEMU_CPU"
gdb=""
for arg; do
	case $arg in
	--gdb) gdb="-s -S" ;;
	--hvf) cpu="-accel hvf -cpu host" ;;
	*) echo "usage: $0 [--gdb] [--hvf]" >&2; exit 2 ;;
	esac
done

if [ -z "${EDK2_FW:-}" ]; then
	share=$(dirname "$(command -v qemu-system-aarch64)")/../share/qemu
	for f in "$share/edk2-aarch64-code.fd" /usr/share/qemu/edk2-aarch64-code.fd \
		/usr/share/qemu-efi-aarch64/QEMU_EFI.fd /usr/share/AAVMF/AAVMF_CODE.fd; do
		if [ -f "$f" ]; then EDK2_FW=$f; break; fi
	done
fi
if [ -z "${EDK2_FW:-}" ]; then
	echo "run-qemu: EDK2 firmware not found, set EDK2_FW" >&2
	exit 1
fi

# The machine options must match the DTB seL4 dumped at configure time
# (seL4/src/plat/qemu-arm-virt/config.cmake). acpi=off makes EDK2 hand the DTB
# to Limine; with ACPI tables present it passes ACPI instead.
exec qemu-system-aarch64 -machine "virt,secure=off,gic-version=$QEMU_GIC,acpi=off" $cpu \
	-smp 1 -m "$QEMU_MEM" -nographic -nic none -bios "$EDK2_FW" \
	-drive "if=virtio,format=raw,file=$root/out/esp.img" $gdb
