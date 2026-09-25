#!/bin/sh
# One-time host setup: installs the cross toolchain, seL4 build tools, mtools
# and QEMU + EDK2, then checks out the seL4 submodule.
set -eu

case "$(uname -s)" in
Darwin)
	brew install aarch64-elf-gcc aarch64-elf-binutils cmake ninja dtc mtools qemu uv
	;;
Linux)
	cc=gcc-aarch64-linux-gnu
	if [ "$(uname -m)" = aarch64 ]; then
		cc=gcc # the native gcc is aarch64-linux-gnu-gcc
	fi
	sudo apt-get update
	sudo apt-get install -y make git curl $cc cmake ninja-build device-tree-compiler \
		libxml2-utils python3 mtools qemu-system-arm qemu-efi-aarch64 gdb-multiarch
	if ! command -v uv >/dev/null; then
		curl -LsSf https://astral.sh/uv/install.sh | sh
		echo "setup-host: uv went to ~/.local/bin, make sure it is on PATH"
	fi
	;;
*)
	echo "unsupported host: $(uname -s)" >&2
	exit 1
	;;
esac

git -C "$(dirname "$0")/.." submodule update --init
