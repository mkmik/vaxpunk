#!/bin/sh
# Downloads the real VMS disk images the tests read, and checks them against
# SHA256SUMS. Images are never committed. Without arguments only the small
# ones (a few MB) are fetched; --all adds the CD images (about 1 GB).
#
#   fixtures/fetch.sh [--all]
set -eu

cd "$(dirname "$0")"
all=false
[ "${1:-}" = --all ] && all=true

ia=https://archive.org/download
# Pinned: a VMS 7.1 INITIALIZE of an empty RA92, from github.com/allenpomeroy/ods2v2 (MIT).
ods2v2=https://raw.githubusercontent.com/allenpomeroy/ods2v2/30068dc351c901847b04e673bfbeb28c4773d5cc

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

# fetch NAME URL: download URL and unpack it into NAME, unless NAME is there.
fetch() {
	name=$1 url=$2
	[ -f "$name" ] && return
	echo "fetch: $name"
	curl -fsSL -o "$tmp/dl" "$url"
	case $url in
	*.7z | *.ZIPEXE)
		mkdir "$tmp/x"
		bsdtar -xf "$tmp/dl" -C "$tmp/x"
		mv "$tmp/x"/* "$name"
		rm -rf "$tmp/x"
		;;
	*.gz) gunzip -c "$tmp/dl" >"$name" ;;
	*.bz2) bunzip2 -c "$tmp/dl" >"$name" ;;
	*) mv "$tmp/dl" "$name" ;;
	esac
}

# Volumes OpenVMS Alpha 8.4-2L1 made, ODS-5 and ODS-2 (vms/samples.dcl):
# small enough compressed to live in the repository.
for f in vms-ods5.img vms-ods2.img; do
	[ -f $f ] || xz -dc $f.xz >$f
done

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

# Check whatever is present.
for f in $(awk '{print $2}' SHA256SUMS); do
	[ -f "$f" ] && grep " $f\$" SHA256SUMS
done | shasum -a 256 -c -
