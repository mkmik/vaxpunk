#!/bin/sh
# Build qemu-system-aarch64 for the browser, as ktock/qemu-wasm's README does, into DIR:
# qemu-system-aarch64.js, .wasm and .worker.js. Needs Docker; takes the better part of an hour.
# Usage: qemu-wasm.sh DIR
set -eux

rev=0ef7b4e2814b231705d8371dd7997f5b72e70baf
out=$(mkdir -p "$1" && cd "$1" && pwd)
src=$(mktemp -d)
git -C "$src" init -q
git -C "$src" fetch -q --depth 1 https://github.com/ktock/qemu-wasm "$rev"
git -C "$src" checkout -q FETCH_HEAD
# zlib.net keeps only the latest release.
sed -i.orig 's#https://zlib.net/zlib-$ZLIB_VERSION.tar.xz#https://github.com/madler/zlib/releases/download/v$ZLIB_VERSION/zlib-$ZLIB_VERSION.tar.xz#' "$src/Dockerfile"

docker build -t qemu-wasm-build - < "$src/Dockerfile"
cflags="-O3 -Wno-error=unused-command-line-argument -matomics -mbulk-memory -DNDEBUG -DG_DISABLE_ASSERT -D_GNU_SOURCE -sASYNCIFY=1 -pthread -sPROXY_TO_PTHREAD=1 -sFORCE_FILESYSTEM -sALLOW_TABLE_GROWTH -sTOTAL_MEMORY=2300MB -sWASM_BIGINT -sMALLOC=mimalloc --js-library=/build/node_modules/xterm-pty/emscripten-pty.js -sEXPORT_ES6=1 -sASYNCIFY_IMPORTS=ffi_call_js"
# configure fetches dtc, for the virt board, into the source tree: a copy, so
# nothing the container's root writes is left in $src for us to delete.
docker run --rm -v "$src":/src:ro -v "$out":/out qemu-wasm-build sh -euxc "
	cp -R /src /qemu
	emconfigure /qemu/configure --static --target-list=aarch64-softmmu --cpu=wasm32 --cross-prefix= \
		--without-default-features --enable-system --with-coroutine=fiber \
		--extra-cflags='$cflags' --extra-cxxflags='$cflags' \
		--extra-ldflags='-sEXPORTED_RUNTIME_METHODS=getTempRet0,setTempRet0,addFunction,removeFunction,TTY,FS'
	emmake make -j\$(nproc) qemu-system-aarch64
	cp qemu-system-aarch64 /out/qemu-system-aarch64.js
	cp qemu-system-aarch64.wasm qemu-system-aarch64.worker.js /out/"
rm -rf "$src"
