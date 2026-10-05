_default:
    @just --list

# Stitches the ESP image and boots it in QEMU, under HVF on a Mac.
boot:
    cargo run -p boot -- {{ if os() == "macos" { "--hvf" } else { "" } }}
