$ ! SYSTARTUP_VMS.COM: the STARTUP process's DCL runs it when the system
$ ! starts, before anyone logs in, as VMS's STARTUP runs it: the site's
$ ! own startup.
$ ! P1 to P8 are the SYSGEN parameters STARTUP_P1 to STARTUP_P8, up to 4
$ ! characters each, as VMS's STARTUP.COM reads them; run-qemu.sh's
$ ! --p1 to --p8 set them, as in cargo run -p boot -- --p1=MIN. Test
$ ! them here to start the system one way or another.
$ IF P1 .EQS. "" THEN P1 = F$EDIT(F$GETSYI("STARTUP_P1"), "TRIM,UPCASE")
$ IF P2 .EQS. "" THEN P2 = F$EDIT(F$GETSYI("STARTUP_P2"), "TRIM,UPCASE")
$ IF P3 .EQS. "" THEN P3 = F$EDIT(F$GETSYI("STARTUP_P3"), "TRIM,UPCASE")
$ IF P4 .EQS. "" THEN P4 = F$EDIT(F$GETSYI("STARTUP_P4"), "TRIM,UPCASE")
$ IF P5 .EQS. "" THEN P5 = F$EDIT(F$GETSYI("STARTUP_P5"), "TRIM,UPCASE")
$ IF P6 .EQS. "" THEN P6 = F$EDIT(F$GETSYI("STARTUP_P6"), "TRIM,UPCASE")
$ IF P7 .EQS. "" THEN P7 = F$EDIT(F$GETSYI("STARTUP_P7"), "TRIM,UPCASE")
$ IF P8 .EQS. "" THEN P8 = F$EDIT(F$GETSYI("STARTUP_P8"), "TRIM,UPCASE")
$ ! TCPIP$DEVICE names the network's template device, which a program
$ ! assigns a channel to for a socket, as TCP/IP Services' TCPIP$STARTUP
$ ! defines it.
$ DEFINE/SYSTEM/NOLOG TCPIP$DEVICE _BGA0:
$ ! It makes the ramdisk, MDA0:, afresh at each boot, and mounts it. It
$ ! holds TCP/IP Services' files: the hosts database, TCPIP$HOST.DAT,
$ ! which the first hosts command makes, with LOCALHOST, and the saved
$ ! network configuration, TCPIP$CONFIG.DAT, which START
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
$ CREATE/DIRECTORY MDA0:[SYSLIB]
$ CREATE/DIRECTORY MDA0:[SYSMGR]
$ DEFINE/SYSTEM/NOLOG SYS$SPECIFIC MDA0:
$ DEFINE/SYSTEM/NOLOG SYS$COMMON SYS$SYSDEVICE:
$ DEFINE/SYSTEM/NOLOG SYS$SYSROOT SYS$SPECIFIC:,SYS$COMMON:
$ DEFINE/SYSTEM/NOLOG SYS$SYSTEM SYS$SYSROOT:[SYSEXE]
$ DEFINE/SYSTEM/NOLOG SYS$LIBRARY SYS$SYSROOT:[SYSLIB]
$ DEFINE/SYSTEM/NOLOG SYS$SHARE SYS$SYSROOT:[SYSLIB]
$ DEFINE/SYSTEM/NOLOG SYS$MANAGER SYS$SYSROOT:[SYSMGR]
$ ! SSL3's certificates, as VSI's SSL3$STARTUP.COM names them: CERT.PEM,
$ ! the CAs SSL3 trusts unless SSL_CERT_FILE names others.
$ DEFINE/SYSTEM/NOLOG SSL3$CERTS SYS$COMMON:[SSL3.CERTS]
$ ! A process's default device, for one no one logged in to.
$ DEFINE/SYSTEM/NOLOG SYS$DISK SYS$SYSROOT:
$ ! The users, SYSUAF.DAT, which LOGINOUT reads, AUTHORIZE and SET
$ ! PASSWORD change: a copy on the ramdisk, which they can write, SYSTEM's
$ ! alone, until the system stops. To keep the users from one boot to the
$ ! next, copy it to the data disk once, and define SYSUAF here instead:
$ !   DEFINE/SYSTEM/NOLOG SYSUAF DKB0:[000000]SYSUAF.DAT
$ COPY SYS$COMMON:[SYSEXE]SYSUAF.DAT SYS$SPECIFIC:[SYSEXE]SYSUAF.DAT
$ SET PROTECTION=(S:RWE,O:RWE,G,W) SYS$SPECIFIC:[SYSEXE]SYSUAF.DAT
$ ! The BIND resolver asks Google's public DNS server, for a host the
$ ! hosts database hasn't: PING, TELNET and nslookup. When DHCP sets the
$ ! interface, below, the DNS server the DHCP server offers takes its
$ ! place, as QEMU's 10.0.2.3, which works where a network or a VPN
$ ! blocks public ones.
$ TCPIP SET NAME_SERVICE /SERVER=8.8.8.8 /SYSTEM
$ ! START COMMUNICATION sets the network interface as TCPIP's SET
$ ! CONFIGURATION INTERFACE and SET ROUTE /PERMANENT saved it on the
$ ! ramdisk, above, or with DHCP, and starts the remote login server,
$ ! TELNETD, whose logins SET HOST makes.
$ TCPIP START COMMUNICATION
$ ! Last, it mounts the data disk, which nothing at boot needs, if an
$ ! INITIALIZE DKB0: made a volume there;
$ ! on a blank one, MOUNT fails with NOHOMEBLK, and the procedure ends.
$ MOUNT DKB0:
