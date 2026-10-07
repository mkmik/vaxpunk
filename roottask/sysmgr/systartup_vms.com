$ ! SYSTARTUP_VMS.COM: DCL runs it when the system starts, before
$ ! SYLOGIN.COM, as VMS's STARTUP runs it: the site's own startup.
$ ! TCPIP$DEVICE names the network's template device, which a program
$ ! assigns a channel to for a socket, as TCP/IP Services' TCPIP$STARTUP
$ ! defines it.
$ DEFINE/SYSTEM/NOLOG TCPIP$DEVICE _BGA0:
$ ! It mounts the data disk, if an INITIALIZE DKB0: made a volume there;
$ ! on a blank one, MOUNT fails with NOHOMEBLK, and the procedure ends.
$ MOUNT DKB0:
