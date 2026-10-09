$ ! SHUTDOWN.COM: stops the system, as VMS's SYS$SYSTEM:SHUTDOWN.COM does,
$ ! by running OPCCRASH last, which takes CMKRNL. Without it OPCCRASH fails
$ ! with SS$_NOPRIV, and the procedure stops there.
$ ! ponytail: no questions, no warning to the other users, no stopping
$ ! their processes first; the file system writes each change as it goes.
$ WRITE SYS$OUTPUT ""
$ WRITE SYS$OUTPUT "        SHUTDOWN -- Perform an Orderly System Shutdown"
$ WRITE SYS$OUTPUT ""
$ MCR OPCCRASH
