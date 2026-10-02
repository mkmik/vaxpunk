_default:
    @just --list

# Stitches the ESP image and boots it in QEMU.
boot:
    cargo run -p boot

# Regenerates the API reference, docs/api/index.html, from the sources.
apidoc:
    scripts/apidoc.py
