#!/bin/sh
# One-time host setup: installs the cross toolchain, seL4 build tools, mtools
# and QEMU + EDK2, then checks out the seL4 submodule.
set -eu

case "$(uname -s)" in
Darwin)
	brew install aarch64-elf-gcc aarch64-elf-binutils cmake ninja dtc mtools qemu uv
	;;
Linux)
	sudo apt-get update
	sudo apt-get install -y gcc-aarch64-linux-gnu cmake ninja-build device-tree-compiler \
		libxml2-utils python3 curl mtools qemu-system-arm qemu-efi-aarch64 gdb-multiarch
	command -v uv >/dev/null || curl -LsSf https://astral.sh/uv/install.sh | sh
	;;
*)
	echo "unsupported host: $(uname -s)" >&2
	exit 1
	;;
esac

git -C "$(dirname "$0")/.." submodule update --init
