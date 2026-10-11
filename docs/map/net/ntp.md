+++
title = "NTP"
summary = "Keeps the system's time from NTP servers"
aka = ["NTP", "TCPIP$NTP", "NTPQ", "NTPDATE", "DTSS"]
kind = "core"
intent = "aspiration"
research = "deferred"
status = "todo"

[[needs]]
on = "/net/tcpip-qio"
why = "UDP on port 123"

[[needs]]
on = "/exec/timers"
why = "Adjusts the system time"
+++

TCP/IP Services runs an NTP daemon that keeps the system clock in step
with NTP servers, with `NTPQ` and `NTPDATE` to query and set it. DECnet
systems used DECdts, the Distributed Time Service, instead.

vaxpunk has none: its time is the virtual machine's real-time clock,
taken as UTC.
