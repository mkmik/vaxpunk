+++
title = "lwIP TCP/IP component"
summary = "lwIP and a virtio-net driver in a seL4 task below the port: a bootstrap stack"
aka = ["lwIP", "tcpip.elf", "pal/tcpip", "TCP/IP component", "the port", "lwipopts.h"]
kind = "platform"
intent = "induced"
research = "done"
milestone = "m0"
refs = ["docs/prd/0002-networking.md", "docs/design/0003-tcpip-port.md", "docs/adr/0016-tcpip-component-and-bga0.md", "docs/adr/0022-tcpip-utility-and-dhcp.md", "pal/tcpip/src/main.c", "pal/tcpip/include/lwipopts.h", "pal/tcpip/include/port.h"]

[[feature]]
id = "component"
title = "lwIP 2.2.1, raw API, NO_SYS, in a task of its own the PAL starts"
status = "done"
milestone = "m0"
note = "ADR-0016 decision 1; one 2 MB page, priority between the PAL and the executive"

[[feature]]
id = "port"
title = "The port: 17 shared pages, command and response rings, a doorbell and a completion interrupt"
status = "done"
milestone = "m0"
note = "PRD-0002; DESIGN-0003; versioned protocol"

[[feature]]
id = "protocols"
title = "IPv4, ARP, ICMP, TCP, UDP and raw ICMP"
status = "done"
note = "lwipopts.h; UDP and raw sockets came with ADR-0024"

[[feature]]
id = "dhcp"
title = "DHCP client, and the DNS server a DHCP server offers"
status = "done"
note = "ADR-0022; PR #144; ponytail: no ARP probe of the offered address, one DNS server kept"

[[feature]]
id = "virtio-net-irq"
title = "virtio-net driven by its own interrupt, through seL4's IRQ handler"
status = "done"
needs = ["//platform/virtio:net"]

[[feature]]
id = "zero-copy"
title = "Frames without copies, and the device's offloads"
status = "todo"
note = "PRD-0003 tier 4 #41; pal/tcpip/src/main.c ponytail"

[[feature]]
id = "own-rights"
title = "A frame per segment with its own rights, not one writable and executable page"
status = "todo"
note = "PRD-0003 tier 4 #41; main.c ponytail on start_tcpip"

[[feature]]
id = "ipv6"
title = "IPv6"
status = "todo"
note = "LWIP_IPV6 0; lwIP has it"

[[feature]]
id = "native-replacement"
title = "Replaced by a native stack in MACRO-32 or BLISS speaking the same port protocol"
status = "todo"
note = "PRD-0002 Replace-later path"

[[needs]]
on = "//platform/pal"
why = "The PAL builds its task, maps the port and relays the doorbell and interrupt"

[[needs]]
on = "//platform/virtio"
why = "Its network interface is a virtio-net device"

[[needs]]
on = "//platform/sel4"
why = "A thread, notification and IRQ handler of its own"

[[needs]]
on = "//host/cross-gcc"
why = "C, built with the bare-metal cross compiler"
+++

OpenVMS's TCP/IP stack is TCP/IP Services (formerly UCX), a BSD-derived
kernel stack that runs inside the executive as drivers (`BGDRIVER`,
`INETACP` and the internet kernel), below the `$QIO` interface programs
see. Other vendors sold their own: Process Software's MultiNet and
TCPware. VMS has nothing like a separate stack task.

vaxpunk took lwIP because TCP/IP needed a stack and writing one first
would have held up the applications
([PRD-0002](../../docs/prd/0002-networking.md)). It is a replaceable
bootstrap: `pal/tcpip` builds `tcpip.elf`, lwIP's raw API with a small
virtio-net driver and an adapter, which the PAL embeds and starts as a
task of its own ([ADR-0016](../../docs/adr/0016-tcpip-component-and-bga0.md)).
The executive talks to it only through the port, a DEC-style ring of
commands and responses in 17 shared pages, with a doorbell and an
interrupt ([DESIGN-0003](../../docs/design/0003-tcpip-port.md)).
Everything above the port is permanent VMS code; a native stack can
replace lwIP by speaking the same protocol. Corners cut: frames are
copied, no offloads, its page is writable and executable, IPv4 only.
