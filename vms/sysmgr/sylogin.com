$ ! SYLOGIN.COM: DCL runs it at each login, before the user's LOGIN.COM,
$ ! as on VMS. HOME goes back to the system manager's directory.
$ HOME == "SET DEFAULT SYS$MANAGER:"
$ ! nslookup, as TCP/IP Services' TCPIP$DEFINE_COMMANDS.COM defines it.
$ NSLOOKUP :== $SYS$SYSTEM:TCPIP$NSLOOKUP.EXE
