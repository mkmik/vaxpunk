+++
title = "SNMP"
summary = "SNMP agent: MIB-II and VMS's own MIBs, for network management"
aka = ["SNMP", "TCPIP$SNMP", "MIB", "eSNMP"]
kind = "core"
intent = "aspiration"
research = "deferred"
status = "todo"

[[needs]]
on = "//net/tcpip-qio"
why = "UDP on port 161"

[[needs]]
on = "//exec/information-services"
why = "Answers from $GETSYI and $GETDVI"
+++

TCP/IP Services includes an SNMP agent, with MIB-II, the host resources
MIB and subagents, and the eSNMP API for writing more, so network
management stations can watch a VMS system.

vaxpunk has none.
