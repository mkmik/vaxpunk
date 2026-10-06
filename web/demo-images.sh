#!/bin/sh
# Gzip the demo's firmware and disks into DIR: out/esp.img and out/sysdisk.img, which cargo run -p
# boot builds, and the EDK2 firmware run-qemu.sh boots, as edk2-aarch64-code.fd.
# Usage: demo-images.sh DIR
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
mkdir -p "$1"
gzip -9c "$root/out/esp.img" >"$1/esp.img.gz"
gzip -9c "$root/out/sysdisk.img" >"$1/sysdisk.img.gz"
gzip -9c "$("$root/scripts/run-qemu.sh" --firmware)" >"$1/edk2-aarch64-code.fd.gz"
