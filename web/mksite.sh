#!/bin/sh
# Put the GitHub Pages site together in SITE: web/index.html at /, the API reference
# (cargo run -p apidoc) at /docs/ and the demo at /demo/, with QEMU from QEMU (qemu-wasm.sh's
# DIR) and the gzipped firmware and disks from IMAGES (boot.yml's demo-images). Needs npm.
# Usage: mksite.sh SITE QEMU IMAGES
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
site=$1 qemu=$2 images=$3
rm -rf "$site"
mkdir -p "$site/demo/vendor"
cp "$root/web/index.html" "$root/apidoc/page.css" "$site/"
cp -R "$root/docs/api" "$site/docs"
cp "$root/web/demo/index.html" "$root/web/demo/lan.js" "$qemu"/qemu-system-aarch64.* "$images"/*.gz "$site/demo/"

npm=$(mktemp -d)
npm i -s --no-audit --no-fund --prefix "$npm" xterm@5.3.0 xterm-pty@0.10.1 coi-serviceworker@0.1.7
cp "$npm/node_modules/xterm/lib/xterm.js" "$npm/node_modules/xterm/css/xterm.css" "$site/demo/vendor/"
cp "$npm/node_modules/xterm-pty/index.js" "$site/demo/vendor/xterm-pty.js"
cp "$npm/node_modules/coi-serviceworker/coi-serviceworker.min.js" "$site/demo/coi-serviceworker.js"
rm -rf "$npm"
