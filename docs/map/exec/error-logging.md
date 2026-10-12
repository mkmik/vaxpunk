+++
title = "Error logging"
summary = "Device errors, machine checks and system events written to ERRLOG.SYS"
aka = ["ERRFMT", "ERRLOG.SYS", "SYS$ERRORLOG", "$SNDERR", "ERL$DEVICERR", "ERL$DEVICTMO", "error log buffer", "ANALYZE/ERROR_LOG", "DECevent", "DIAGNOSE", "Error Log Viewer", "ELV", "error count"]
kind = "core"
intent = "aspiration"
research = "outline"
refs = ["vms/exec/f11.mar", "vms/exec/getdvi.mar"]

[[feature]]
id = "buffers"
title = "Error log buffers in the executive, which drivers and the executive fill"
status = "todo"

[[feature]]
id = "device-errors"
title = "Device errors and timeouts logged by drivers: ERL$DEVICERR, ERL$DEVICTMO"
status = "todo"
needs = ["#buffers"]

[[feature]]
id = "errfmt"
title = "ERRFMT, the process that writes the buffers to SYS$ERRORLOG:ERRLOG.SYS"
status = "todo"
needs = ["#buffers"]

[[feature]]
id = "events"
title = "System events: startup, mounts and dismounts, bugchecks, time changes"
status = "todo"
needs = ["#buffers"]

[[feature]]
id = "snderr"
title = "$SNDERR: a privileged program's own message in the error log"
status = "todo"
needs = ["#buffers"]

[[feature]]
id = "machine-checks"
title = "Machine checks and corrected errors from the hardware"
status = "todo"
needs = ["/platform/real-hardware"]

[[feature]]
id = "analyze"
title = "ANALYZE/ERROR_LOG, and later DECevent and the Error Log Viewer"
status = "todo"
needs = ["#errfmt"]
note = "/utils/error-log"

[[needs]]
on = "/utils/error-log"
why = "ANALYZE/ERROR_LOG reports from ERRLOG.SYS"
soft = true

[[needs]]
on = "/exec/qio"
why = "Drivers log errors from their I/O completion paths"

[[needs]]
on = "/files/ods2"
why = "ERRFMT appends to ERRLOG.SYS"

[[needs]]
on = "/exec/crash-dump"
why = "Nonfatal bugchecks are logged"
soft = true

[[needs]]
fuzzy = "The error log record format: headers, entry types, device-specific packets"
why = "ANALYZE/ERROR_LOG and DECevent read the records"
+++

VMS logs hardware and system errors. When a driver sees a device error
or timeout it copies the device's registers into an error log buffer in
the executive and counts the error in the UCB; the executive does the
same for machine checks, bugchecks, volume mounts and system startup.
The `ERRFMT` process writes the buffers to
`SYS$ERRORLOG:ERRLOG.SYS`. `ANALYZE/ERROR_LOG` reports on the file, and
field service used it, then DECevent and the Error Log Viewer, to find
failing disks and memory before they failed for good. `$SNDERR` lets a
privileged program add its own message. `SHOW DEVICES` shows each
device's error count.

vaxpunk counts a disk's failed transfers in the UCB's `UCB$L_ERRCNT`
(`vms/exec/f11.mar`), which `$GETDVI`'s `DVI$_ERRCNT` and `SHOW
DEVICES` show, and nothing more: no error log buffers, `ERRFMT` or
`ERRLOG.SYS`. On QEMU there are few hardware errors to log.
