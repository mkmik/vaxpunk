$ ! SYLOGIN.COM: DCL runs it when SYSTEM starts, as VMS runs it at each
$ ! login. HOME goes back to the system manager's directory.
$ HOME == "SET DEFAULT SYS$MANAGER:"
