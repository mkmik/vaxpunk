+++
title = "SLIP and PPP"
summary = "IP over serial lines and modems"
aka = ["SLIP", "PPP", "CSLIP", "PPPD", "SET INTERFACE SL0"]
kind = "core"
intent = "aspiration"
research = "deferred"
status = "todo"

[[needs]]
on = "/drivers/terminal"
why = "Runs over a terminal line"

[[needs]]
fuzzy = "A serial line driver and an interface below the port that isn't Ethernet"
why = "lwIP's component knows only virtio-net"
+++

TCP/IP Services runs IP over terminal lines: SLIP, compressed SLIP
and PPP, for dial-up links and point-to-point connections,
configured as interfaces such as `SL0` and `PP0`.

vaxpunk has no serial lines beyond the console.
