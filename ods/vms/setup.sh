#!/bin/sh
# Builds the AXPbox Alpha emulator (GPL-2.0, cloned here, never committed)
# and fetches the AlphaServer ES40 SRM firmware it boots, then checks that
# the OpenVMS Alpha 8.4-2L1 install CD is among the fixtures.
#
#   vms/setup.sh
set -eu

cd "$(dirname "$0")"
axpbox=ab554ae3ce2d77933d5886a63c0a4d4f646910ce # v1.2.0
rom=392546cd375a734883a48026d36f6f48e3b0fce636f6dd9ee190d49a042fc885

if [ ! -x axpbox/build/axpbox ]; then
	[ -d axpbox ] || git clone -q https://github.com/lenticularis39/axpbox axpbox
	git -C axpbox checkout -q "$axpbox"
	cmake -S axpbox -B axpbox/build -G Ninja -DCMAKE_BUILD_TYPE=Release \
		-DDISABLE_SDL=yes -DDISABLE_X11=yes -DDISABLE_PCAP=yes >/dev/null
	cmake --build axpbox/build >/dev/null
fi

mkdir -p rom
if [ ! -f rom/cl67srmrom.exe ]; then
	# The URL AXPbox's own OpenVMS guide and tests use.
	curl -fsSL -o rom/cl67srmrom.exe http://raymii.org/s/inc/downloads/es40-srmon/cl67srmrom.exe
fi
echo "$rom  rom/cl67srmrom.exe" | shasum -a 256 -c - >/dev/null

if [ ! -f ../fixtures/alpha-8.4-2l1.iso ]; then
	echo "setup: the OpenVMS CD is missing: run fixtures/fetch.sh --all" >&2
	exit 1
fi
echo "setup: ready"
