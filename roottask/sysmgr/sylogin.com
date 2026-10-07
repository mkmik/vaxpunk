$ ! SYLOGIN.COM: DCL runs it when SYSTEM starts, as VMS runs it at each
$ ! login. HOME goes back to the system manager's directory.
$ HOME == "SET DEFAULT SYS$MANAGER:"
$ ! START COMMUNICATION sets the network interface as TCPIP's SET
$ ! CONFIGURATION INTERFACE and SET ROUTE /PERMANENT saved it on the
$ ! ramdisk, in SYSTARTUP_VMS.COM, or with DHCP, and starts the remote
$ ! login server.
$ TCPIP START COMMUNICATION
