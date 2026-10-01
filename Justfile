_default:
    @just --list

# Stitches the ESP image and boots it in QEMU.
boot:
    cargo run -p boot

# Boots without a terminal, types commands at the console's DCL prompt
# once the executive has started, and waits until the processes have
# printed their last lines (roottask/sysexe/), or the root task is done,
# which is only on a halt or a fault. DCL reads what was typed ahead a line
# at a time. The CPU then idles, taking clock interrupts.
check:
    #!/usr/bin/env bash
    set -u
    cargo build -q -p boot || exit 1
    rm -f out/serial.log
    fifo=$(mktemp -u)
    mkfifo "$fifo"
    cargo run -q -p boot < "$fifo" > /dev/null 2>&1 &
    exec 3> "$fifo"
    rm "$fifo"
    lines=('STARTUP: done' 'SVCTEST: ok' 'process NOSUCH exited' 'HOG: NUDGE ran' 'TIMETEST: ok'
           'process SNOOP exited with status 0000000C' 'process USURP exited with status 0000043C'
           '%NONAME-F-NOMSG, Message number 0000000C' ' \FOO\' 'Total of 2 files, 10 blocks.')
    all() { for line in "${lines[@]}"; do grep -aqsF "$line" out/serial.log || return 1; done; }
    typed=
    for _ in $(seq 120); do
        if [ -z "$typed" ] && grep -aqs '%EXEC-I-START' out/serial.log; then
            printf 'RUN STARTUP\rRUN SNOOP\rFOO\rDIR P%%NG\r' >&3
            typed=1
        fi
        all || grep -aqs 'root task done' out/serial.log && break
        sleep 1
    done
    pkill -f "qemu-system-aarch64.*$PWD/out/esp.img"
    exec 3>&-
    tr -d '\r' < out/serial.log | sed -n '/EXEC.EXE:/,$p'
    for line in "${lines[@]}"; do
        grep -aqF "$line" out/serial.log || { echo "check: no \"$line\"" >&2; exit 1; }
    done

# Regenerates the API reference, docs/api/index.html, from the sources.
apidoc:
    scripts/apidoc.py
