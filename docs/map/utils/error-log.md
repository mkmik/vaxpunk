+++
title = "ANALYZE/ERROR_LOG"
summary = "Reports from ERRLOG.SYS: device errors, machine checks, volume mounts"
aka = ["ANALYZE/ERROR_LOG", "ERF", "ELV", "ERRFMT", "ERRLOG.SYS", "SYS$ERRORLOG", "DECevent", "WEBES", "SHOW ERROR"]
kind = "core"
intent = "aspiration"
research = "deferred"
refs = ["https://wasd.vsm.com.au/htbin/helpgate/HELP/ANALYZE/_ERROR_LOG"]

[[feature]]
id = "report"
title = "ANALYZE/ERROR_LOG: entries by device, type and time"
status = "todo"
needs = ["/exec/error-logging#errfmt"]

[[feature]]
id = "elv"
title = "ANALYZE/ERROR_LOG/ELV: the Error Log Viewer's commands and translation"
status = "todo"
needs = ["#report"]

[[needs]]
on = "/exec/error-logging"
why = "The executive's error log buffers, which ERRFMT writes to ERRLOG.SYS"

[[needs]]
on = "/rms/sequential"
why = "Reads SYS$ERRORLOG:ERRLOG.SYS"
+++

Drivers and the executive record device errors, timeouts, machine
checks, volume mounts and dismounts, and system startups in error log
buffers; the `ERRFMT` process copies them into
`SYS$ERRORLOG:ERRLOG.SYS`, and `SHOW ERROR` counts them per device.
`ANALYZE/ERROR_LOG` reads the file and prints the entries, decoded,
selected by device, type and time. On Alpha the old formatter gave way
to DECevent and then to WEBES and the Error Log Viewer
(`ANALYZE/ERROR_LOG/ELV`), which also translate the newer binary
records for field service.

vaxpunk has no error log. Under seL4 and QEMU the devices are virtio,
and errors show as console lines or failed I/O status; an error log
matters once there is real hardware.
