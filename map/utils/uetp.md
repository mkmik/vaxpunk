+++
title = "UETP"
summary = "The User Environment Test Package: SYS$TEST:UETP.COM exercises a new system"
aka = ["UETP", "UETP.COM", "SYS$TEST", "User Environment Test Package", "SYSTEST account"]
kind = "core"
intent = "aspiration"
research = "deferred"
refs = ["docs/boot.md", "vms/sysexe/startup.mar"]

[[feature]]
id = "phases"
title = "@SYS$TEST:UETP: initialization, device, system load and cleanup phases"
status = "todo"
needs = ["//dcl/procedures", "//exec/processes"]

[[feature]]
id = "devices"
title = "Each disk, tape and terminal tested with reads and writes"
status = "todo"
needs = [":phases", "//exec/information-services"]

[[feature]]
id = "load"
title = "A load test: many detached processes doing I/O and computing at once"
status = "todo"
needs = [":phases"]

[[needs]]
on = "//dcl/procedures"
why = "UETP.COM drives the tests"

[[needs]]
on = "//exec/processes"
why = "Each test and load process is a process of its own"
+++

The User Environment Test Package is how DEC and system managers
checked a new or repaired VMS system. Logged in to the `SYSTEST`
account, `@SYS$TEST:UETP` runs phases: it reads and writes every disk,
tape and terminal it finds, loads the system with many processes doing
I/O and computing for a while, runs the network tests, and cleans up,
logging what failed. It ships with the base system in `SYS$TEST`.

vaxpunk has no UETP. The nearest thing is `RUN STARTUP`
(`vms/sysexe/startup.mar`), which starts test images that exercise
the executive (`SVCTEST`, `ASTTEST`, `MBXTEST`, `FSTEST`, `CHFTEST`,
`TIMETEST`), and the host's `cargo test -p boot`, which drives a whole
boot from a transcript ([boot.md](../../docs/boot.md)).
