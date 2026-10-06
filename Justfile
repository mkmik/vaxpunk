_default:
    @just --list

# Stitches the ESP image and boots it in QEMU, under HVF on a Mac.
boot:
    cargo run -p boot -- {{ if os() == "macos" { "--hvf" } else { "" } }}

# Builds the web demo into out/site, serves it on localhost and opens it. QEMU for the browser is built in Docker the first time, about an hour.
wasm-demo port="8765":
    #!/bin/sh
    set -eu
    cargo run -p boot -- --images
    web/demo-images.sh out/demo
    cargo run -p apidoc
    cache={{ if os() == "macos" { "$HOME/Library/Caches" } else { "${XDG_CACHE_HOME:-$HOME/.cache}" } }}/vaxpunk
    qemu=$cache/qemu-wasm-$(git hash-object web/qemu-wasm.sh)
    [ -f "$qemu/qemu-system-aarch64.wasm" ] || web/qemu-wasm.sh "$qemu"
    web/mksite.sh out/site "$qemu" out/demo
    (sleep 1 && {{ if os() == "macos" { "open" } else { "xdg-open" } }} http://localhost:{{port}}/demo/) &
    exec python3 -m http.server -b localhost -d out/site {{port}}
