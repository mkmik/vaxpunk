#!/bin/sh
# Download the pinned Limine binary release into third_party/limine/.
set -eu

VERSION=11.4.1
SHA256=5a8e5b60858a18eb1493ae6552aa82413ce8f0b0e0c490e5a3e14135839d0e65
URL=https://raw.githubusercontent.com/limine-bootloader/limine/v$VERSION-binary/BOOTAA64.EFI

dir=$(dirname "$0")/../third_party/limine
mkdir -p "$dir"
curl -fsSL -o "$dir/BOOTAA64.EFI.part" "$URL"
if command -v sha256sum >/dev/null; then
	sum=$(sha256sum "$dir/BOOTAA64.EFI.part")
else
	sum=$(shasum -a 256 "$dir/BOOTAA64.EFI.part")
fi
if [ "${sum%% *}" != "$SHA256" ]; then
	rm -f "$dir/BOOTAA64.EFI.part"
	echo "fetch-limine: checksum mismatch for $URL" >&2
	exit 1
fi
mv "$dir/BOOTAA64.EFI.part" "$dir/BOOTAA64.EFI"
