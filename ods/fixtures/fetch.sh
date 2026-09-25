#!/bin/sh
# Gets the real VMS disk images the tests read, checks them against
# SHA256SUMS and links them here. The images themselves live in
# vaxpunk/fixtures in the user's cache directory (~/Library/Caches on macOS,
# else $XDG_CACHE_HOME or ~/.cache), read-only and named by checksum, so
# every checkout shares one copy; they are never committed. Without
# arguments only the small downloads (a few MB) are fetched; --all adds the
# CD images (about 1 GB).
#
#   fixtures/fetch.sh [--all]
set -eu

cd "$(dirname "$0")"
all=false
[ "${1:-}" = --all ] && all=true

case $(uname) in
Darwin) cache=$HOME/Library/Caches ;;
*) cache=${XDG_CACHE_HOME:-$HOME/.cache} ;;
esac
cache=$cache/vaxpunk/fixtures
mkdir -p "$cache"

ia=https://archive.org/download
# Pinned: a VMS 7.1 INITIALIZE of an empty RA92, from github.com/allenpomeroy/ods2v2 (MIT).
ods2v2=https://raw.githubusercontent.com/allenpomeroy/ods2v2/30068dc351c901847b04e673bfbeb28c4773d5cc

# In the cache's file system, so that images move into it whole.
tmp=$(mktemp -d "$cache/tmp.XXXXXX")
trap 'rm -rf "$tmp"' EXIT
trap 'exit 1' INT TERM

# fetch NAME SOURCE: unless the cache has NAME, get it from SOURCE, a URL or
# a file here, unpack and check it; then link NAME here to the cached copy.
fetch() {
	name=$1 src=$2
	sum=$(awk -v n="$name" '$2 == n { print $1 }' SHA256SUMS)
	if [ ! -f "$cache/$sum" ]; then
		if [ -f "$name" ] && [ ! -L "$name" ]; then
			mv "$name" "$tmp/img" # fetched here before there was a cache
		else
			echo "fetch: $name"
			case $src in
			http*) curl -fsSL -o "$tmp/dl" "$src" ;;
			*) cp "$src" "$tmp/dl" ;;
			esac
			case $src in
			*.7z | *.ZIPEXE)
				mkdir "$tmp/x"
				bsdtar -xf "$tmp/dl" -C "$tmp/x"
				mv "$tmp/x"/* "$tmp/img"
				rm -rf "$tmp/x"
				;;
			*.gz) gunzip -c "$tmp/dl" >"$tmp/img" ;;
			*.bz2) bunzip2 -c "$tmp/dl" >"$tmp/img" ;;
			*.xz) xz -dc "$tmp/dl" >"$tmp/img" ;;
			*) mv "$tmp/dl" "$tmp/img" ;;
			esac
		fi
		got=$(shasum -a 256 <"$tmp/img" | cut -c1-64)
		if [ "$got" != "$sum" ]; then
			echo "fetch: $name has SHA-256 $got, SHA256SUMS wants $sum" >&2
			exit 1
		fi
		chmod a-w "$tmp/img"
		mv "$tmp/img" "$cache/$sum"
	fi
	ln -sf "$cache/$sum" "$name"
}

# Volumes OpenVMS Alpha 8.4-2L1 made, ODS-5 and ODS-2 (vms/samples.dcl):
# small enough compressed to live in the repository.
fetch vms-ods5.img vms-ods5.img.xz
fetch vms-ods2.img vms-ods2.img.xz

# simh RK07 disks: a VMS V1.0 (1978) system disk, and a user disk.
fetch vaxvms-v1.0.rk07 "$ia/Vaxorcist_vax-vms-v-1.0/VAX-VMS_V1.0.RK7.7z"
fetch dungeon.rk07 "$ia/Vaxorcist_vax-vms-v-1.0/DUNGEON.RK7.7z"
fetch vms-7.1-init.dsk "$ods2v2/samples/empty_ods2_volume.dsk.gz"

if $all; then
	# Distribution CDs, ODS-2: VAX/VMS 5.5-2, OpenVMS VAX 6.0, OpenVMS Alpha 8.4-2L1.
	fetch vms-5.5-2.iso "$ia/open-vms/OpenVMS/VAX_5_5_2/vax_vms552.img.gz"
	fetch vms-6.0.iso "$ia/open-vms/OpenVMS/VAX_6_0/openvms_vax_6.0.iso.bz2"
	fetch alpha-8.4-2l1.iso "$ia/alpha-0842-l-1/ALPHA0842L1.ZIPEXE"
fi
