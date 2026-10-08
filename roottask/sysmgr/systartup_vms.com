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
$ ! The BIND resolver asks Google's public DNS server, for a host the
$ ! hosts database hasn't: PING, TELNET and nslookup.
$ TCPIP SET NAME_SERVICE /SERVER=8.8.8.8 /SYSTEM
$ ! Last, it mounts the data disk, which nothing at boot needs, if an
$ ! INITIALIZE DKB0: made a volume there;
$ ! on a blank one, MOUNT fails with NOHOMEBLK, and the procedure ends.
$ MOUNT DKB0:
