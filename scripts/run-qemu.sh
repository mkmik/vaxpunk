#!/bin/sh
# Boot out/esp.img: EDK2 -> Limine -> shim -> seL4 -> root task.
# Usage: run-qemu.sh [--gdb] [--hvf] [--uart1[=PORT]] | --firmware. Quit with Ctrl-A x.
#   --firmware  print the EDK2 firmware image's path and exit
#   --gdb    wait for a debugger on localhost:1234 (-s -S)
#   --hvf    use Hypervisor.framework instead of TCG (best effort, macOS only)
#   --uart1  serve the second UART on telnet localhost:PORT (default 4444)
# EDK2_FW overrides the firmware image, DATADISK the data disk's, LOG the
# console log's, NETDEV the network's QEMU -netdev (QEMU's user network,
# slirp, by default) and MAC the network device's address: a second
# vaxpunk on the same network needs its own of the last four.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
. "$root/kernel/qemu.env"

cpu="-cpu $QEMU_CPU"
gdb=""
uart1=null
firmware=""
for arg; do
	case $arg in
	--gdb) gdb="-s -S" ;;
	--hvf) cpu="-accel hvf -cpu host" ;;
	--firmware) firmware=1 ;;
	--uart1) uart1=telnet:localhost:4444,server,nowait ;;
	--uart1=*) uart1=telnet:localhost:${arg#--uart1=},server,nowait ;;
	*) echo "usage: $0 [--gdb] [--hvf] [--uart1[=PORT]] | --firmware" >&2; exit 2 ;;
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
if [ "$firmware" ]; then echo "$EDK2_FW"; exit; fi

# The machine options must match the DTB seL4 dumped at configure time
# (seL4/src/plat/qemu-arm-virt/config.cmake). acpi=off makes EDK2 hand the DTB
# to Limine; with ACPI tables present it passes ACPI instead. splash-time=0
# drops EDK2's 5 second boot timeout. The console chardev is what -nographic
# sets up (serial and monitor muxed on stdio, Ctrl-C goes to the guest) plus
# a raw log in out/serial.log. serial-filter.py keeps EDK2's and Limine's
# screen control off the terminal.
# The second -serial is UART1 (PL011 at 0x09040000, SPI 8). It always exists,
# so the guest sees the same machine with or without --uart1; without it the
# UART goes nowhere. seL4's DTB lacks it, which is fine: the kernel never uses
# it, and the root task gets its page in the device untyped at 0x9000000.
# The system disk, out/sysdisk.img, is a read-only virtio-blk on virt's
# virtio-mmio transports, which the root task drives itself: modern virtio
# (force-legacy=false), as it expects. The data disk, DKB0:, is another,
# out/datadisk.img, which may be written: 2 MB of zeros the first time,
# until INITIALIZE DKB0: writes a volume on it, then kept from boot to
# boot. They are unit 0 and 1 in the order of their -device options. The
# ESP stays on PCI, for EDK2. Its bootindex makes it the one device EDK2
# connects: given a boot order it leaves the others alone, rather than
# binding its own network stack to virtio-net, whose timer callbacks now
# and then fault on a NULL pointer while Limine loads (DpcDxe, VirtioNetDxe).
# The network device, virtio-net, is a third virtio-mmio transport, after
# the disks, which the TCP/IP component drives. On QEMU's user network the
# guest is 10.0.2.15/24 and the host 10.0.2.2, its gateway, which QEMU's
# DHCP server gives.
# On a halt the root task prints %PAL-I-POWEROFF, and serial-filter.py ends
# QEMU with SIGTERM, by the PID in a file of its own (ADR-0008).
datadisk=${DATADISK:-$root/out/datadisk.img}
pidfile=$(mktemp)
[ -f "$datadisk" ] || dd if=/dev/zero of="$datadisk" bs=512 count=4096 2>/dev/null
qemu-system-aarch64 -machine "virt,secure=off,gic-version=$QEMU_GIC,acpi=off" $cpu \
	-smp 1 -m "$QEMU_MEM" -display none -bios "$EDK2_FW" \
	-boot menu=on,splash-time=0 -drive "if=none,id=esp,format=raw,readonly=on,file=$root/out/esp.img" \
	-device virtio-blk-pci,drive=esp,bootindex=0 \
	-drive "if=none,id=sysdisk,format=raw,readonly=on,file=$root/out/sysdisk.img" \
	-device virtio-blk-device,drive=sysdisk \
	-drive "if=none,id=datadisk,format=raw,file=$datadisk" \
	-device virtio-blk-device,drive=datadisk -global virtio-mmio.force-legacy=false \
	-netdev "${NETDEV:-user,id=net0}" -device "virtio-net-device,netdev=net0${MAC:+,mac=$MAC}" \
	-chardev "stdio,id=con,mux=on,signal=off,logfile=${LOG:-$root/out/serial.log}" \
	-serial chardev:con -serial "$uart1" -monitor chardev:con \
	-pidfile "$pidfile" $gdb | "$root/scripts/serial-filter.py" "$pidfile"
