_default:
    @just --list

# Stitches the ESP image and boots it in QEMU.
boot:
    cargo run -p boot

# Boots without a console until the executive's processes have printed
# their last lines (roottask/sysexe/), or the root task is done, which is
# only on a halt or a fault. The CPU then idles, taking clock interrupts.
check:
    #!/usr/bin/env bash
    set -u
    cargo build -q -p boot || exit 1
    rm -f out/serial.log
    cargo run -q -p boot < /dev/null > /dev/null 2>&1 &
    lines=('STARTUP: done' 'SVCTEST: ok' 'process NOSUCH exited' 'HOG: NUDGE ran' 'TIMETEST: ok')
    all() { for line in "${lines[@]}"; do grep -aqs "$line" out/serial.log || return 1; done; }
    for _ in $(seq 120); do
        all || grep -aqs 'root task done' out/serial.log && break
        sleep 1
    done
    pkill -f "qemu-system-aarch64.*$PWD/out/esp.img"
    tr -d '\r' < out/serial.log | sed -n '/EXEC.EXE:/,$p'
    for line in "${lines[@]}"; do
        grep -aq "$line" out/serial.log || { echo "check: no \"$line\"" >&2; exit 1; }
    done

# Regenerates the API reference, docs/api/index.html, from the sources.
apidoc:
    scripts/apidoc.py
