$ ! SYLOGIN.COM: DCL runs it when SYSTEM starts, as VMS runs it at each
$ ! login. HOME goes back to the system manager's directory.
$ HOME == "SET DEFAULT SYS$MANAGER:"
$ ! TCPIP.EXE, with no command, sets the network interface as SET
$ ! CONFIGURATION INTERFACE and SET ROUTE /PERMANENT last saved it on the
$ ! data disk.
$ RUN TCPIP
