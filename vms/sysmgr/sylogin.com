$ ! SYLOGIN.COM: DCL runs it at each login, before the user's LOGIN.COM,
$ ! as on VMS. HOME goes back to the system manager's directory.
$ HOME == "SET DEFAULT SYS$MANAGER:"
$ ! nslookup, as TCP/IP Services' TCPIP$DEFINE_COMMANDS.COM defines it.
$ NSLOOKUP :== $SYS$SYSTEM:TCPIP$NSLOOKUP.EXE
$ ! The console can't say its window's size, as TELNET's NAWS does: ask it.
$ IF F$GETDVI("SYS$COMMAND","DEVNAM") .EQS. "_OPA0:" THEN RUN SYS$SYSTEM:TTSIZE
$ ! VMS has no SHUTDOWN verb: sites define one for SYS$SYSTEM:SHUTDOWN.COM.
$ SHUTDOWN :== @SYS$SYSTEM:SHUTDOWN
