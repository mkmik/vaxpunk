$ ! SYSTARTUP_VMS.COM: DCL runs it when the system starts, before
$ ! SYLOGIN.COM, as VMS's STARTUP runs it: the site's own startup.
$ ! TCPIP$DEVICE names the network's template device, which a program
$ ! assigns a channel to for a socket, as TCP/IP Services' TCPIP$STARTUP
$ ! defines it.
$ DEFINE/SYSTEM/NOLOG TCPIP$DEVICE _BGA0:
$ ! It makes the ramdisk, MDA0:, afresh at each boot, and mounts it. It
$ ! holds TCP/IP Services' files: the hosts database, TCPIP$HOST.DAT,
$ ! which the first hosts command makes, with LOCALHOST, and the saved
$ ! network configuration, TCPIP$CONFIG.DAT, which SYLOGIN.COM's START
$ ! COMMUNICATION applies, or DHCP without one. Hosts and a fixed address
$ ! for this boot go after the MOUNT, before the data disk's, as
$ !   TCPIP SET HOST "name" /ADDRESS=a.b.c.d /ALIAS="other"
$ !   TCPIP SET CONFIGURATION INTERFACE WE0 /HOST=a.b.c.d /NETWORK_MASK=m.m.m.m
$ !   TCPIP SET ROUTE /DEFAULT /GATEWAY=g.g.g.g /PERMANENT
$ INITIALIZE MDA0: RAM
$ MOUNT MDA0: RAM
$ ! SYS$SYSROOT is a search list, as on a VMScluster's common system
$ ! disk: this system's own root, SYS$SPECIFIC, on the ramdisk, then the
$ ! root every system shares, SYS$COMMON, the write-locked system disk.
$ ! A file is found in the first root that has it, and a new one goes in
$ ! SYS$SPECIFIC, so a system's own SYSTARTUP_VMS.COM or images there
$ ! take the place of the common ones. VMS's roots are directories,
$ ! [SYS0.] and [SYS0.SYSCOMMON.]; here they are whole volumes.
$ CREATE/DIRECTORY MDA0:[SYSEXE]
$ CREATE/DIRECTORY MDA0:[SYSMGR]
$ DEFINE/SYSTEM/NOLOG SYS$SPECIFIC MDA0:
$ DEFINE/SYSTEM/NOLOG SYS$COMMON SYS$SYSDEVICE:
$ DEFINE/SYSTEM/NOLOG SYS$SYSROOT SYS$SPECIFIC:,SYS$COMMON:
$ DEFINE/SYSTEM/NOLOG SYS$SYSTEM SYS$SYSROOT:[SYSEXE]
$ DEFINE/SYSTEM/NOLOG SYS$MANAGER SYS$SYSROOT:[SYSMGR]
$ ! A process's default device, until it sets its own, as the UAF gives
$ ! SYSTEM's on VMS: SHOW DEFAULT says SYS$SYSROOT:[SYSMGR].
$ DEFINE/SYSTEM/NOLOG SYS$DISK SYS$SYSROOT:
$ ! The BIND resolver asks Google's public DNS server, for a host the
$ ! hosts database hasn't: PING, TELNET and nslookup.
$ TCPIP SET NAME_SERVICE /SERVER=8.8.8.8 /SYSTEM
$ ! Last, it mounts the data disk, which nothing at boot needs, if an
$ ! INITIALIZE DKB0: made a volume there;
$ ! on a blank one, MOUNT fails with NOHOMEBLK, and the procedure ends.
$ MOUNT DKB0:
