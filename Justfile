_default:
    @just --list

# Stitches the ESP image and boots it in QEMU.
boot:
    cargo run -p boot

# Boots without a terminal, types commands at the console's DCL prompt
# once the executive has started, and waits until the processes have
# printed their last lines (roottask/sysexe/), and DIRECTORY, TYPE and
# EDIT theirs from the system disk, or the root task is done,
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
    lines=('%MOUNT-I-MOUNTED, VAXPUNK mounted on _DKA0:' 'STARTUP: done' 'SVCTEST: ok'
           'process NOSUCH exited' 'HOG: NUDGE ran' 'TIMETEST: ok'
           'process SNOOP exited with status 0000000C' 'process USURP exited with status 0000043C'
           '%NONAME-F-NOMSG, Message number 0000000C' ' \FOO\' 'PING.EXE;1          PONG.EXE;1'
           'Total of 2 files.' 'and the rest of what INITIALIZE made.'
           $'    9\tand DIRECTORY [000000]' 'String was not found'
           '"FOO" = "SYS$INPUT" (LNM$PROCESS_TABLE)' 'no translation for logical name FOO'
           '(LNM$SYSTEM_TABLE)' '  "SYS$ERROR" = "_OPA0:"' '  DKA0:[SYSMGR]' '  DKA0:[SYSEXE]'
           'DCL.EXE;1           DIRECTORY.EXE;1' '%DCL-I-INVDEF, DKA0:[NOSUCH] does not exist'
           '  DKA0:[NOSUCH]' '  DKA0:[000000]' 'Directory DKA0:[SYSMGR]' 'WELCOME.TXT;1'
           '%NONAME-F-NOMSG, Message number 000184CC')
    all() { for line in "${lines[@]}"; do grep -aqsF "$line" out/serial.log || return 1; done; }
    typed=
    for _ in $(seq 120); do
        if [ -z "$typed" ] && grep -aqs '%EXEC-I-START' out/serial.log; then
            printf 'RUN STARTUP\rRUN SNOOP\rFOO\rDIR [SYSEXE]P%%NG\rTYPE WELCOME.TXT\rEDIT WELCOME.TXT\r"index"\r"zzz"\rQUIT\r' >&3
            typed=1
        fi
        # The rest once EDIT is done: the type-ahead buffer holds 255 characters.
        if [ "$typed" = 1 ] && grep -aqs 'String was not found' out/serial.log; then
            printf 'DEFINE FOO SYS$INPUT\rSHOW LOGICAL FOO\rSHOW LOGICAL\rDEASSIGN FOO\rSHOW LOGICAL FOO\rSHOW DEFAULT\rSET DEFAULT [SYSEXE]\rSHOW DEFAULT\rDIR D*\rSET DEFAULT [NOSUCH]\rSHOW DEFAULT\rSET DEFAULT [-]\rSHOW DEFAULT\rDIR [.SYSMGR]W*\rSET DEFAULT [-]\r' >&3
            typed=2
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
