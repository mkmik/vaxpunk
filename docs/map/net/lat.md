+++
title = "LAT"
summary = "Local Area Transport: terminals and printers on terminal servers"
aka = ["LAT", "LATCP", "LATACP", "LTDRIVER", "LTA", "SET HOST/LAT"]
kind = "core"
intent = "aspiration"
research = "deferred"
status = "todo"

[[needs]]
on = "/drivers/terminal"
why = "Each LAT session is a terminal, LTAn:"

[[needs]]
on = "/drivers/network"
why = "LAT runs straight on Ethernet"

[[needs]]
fuzzy = "Raw Ethernet frames through the port: LAT isn't IP, and the component passes only IP"
why = "LAT's frames, protocol type 0x6004"
+++

LAT, the Local Area Transport, connects terminals and printers on DEC
terminal servers (DECserver) to VMS systems over Ethernet, without IP.
Each session is a terminal device, `LTAn:`, driven by `LTDRIVER` and
managed with `LATCP`; services can be offered by several nodes of a
cluster, and `SET HOST/LAT` reaches another system's service. It is
part of the base system.

vaxpunk has none. The TCP/IP component below the port handles IP only,
so LAT would need a different kind of port message.
