_default:
    @just --list

# Stitches the ESP image and boots it in QEMU.
boot:
    cargo run -p boot

# Boots without a console until the root task is done, then checks that
# the executive's processes ran to the end (roottask/sysexe/).
check:
    #!/usr/bin/env bash
    set -u
    cargo build -q -p boot || exit 1
    rm -f out/serial.log
    cargo run -q -p boot < /dev/null > /dev/null 2>&1 &
    for _ in $(seq 120); do
        grep -aqs 'root task done' out/serial.log && break
        sleep 1
    done
    pkill -f "qemu-system-aarch64.*$PWD/out/esp.img"
    tr -d '\r' < out/serial.log | sed -n '/EXEC.EXE:/,$p'
    for line in 'STARTUP: done' 'SVCTEST: ok' 'PAL-I-IDLE'; do
        grep -aq "$line" out/serial.log || { echo "check: no \"$line\"" >&2; exit 1; }
    done
