+++
title = "Tailnet node"
summary = "A from-scratch Tailscale-compatible node, for a Headscale tailnet"
aka = ["Tailscale", "Headscale", "WireGuard", "DERP", "tsnet"]
kind = "glue"
intent = "aspiration"
research = "deferred"
status = "todo"

[[needs]]
on = "/net/tcpip-qio"
why = "WireGuard runs over UDP; the control plane over TCP"

[[needs]]
on = "/net/ssl"
why = "The coordination server and DERP relays are reached over https"

[[needs]]
on = "/exec/entropy"
why = "Node and WireGuard keys"

[[needs]]
fuzzy = "WireGuard and the Noise protocol, Curve25519 and ChaCha20-Poly1305, and Tailscale's control protocol as Headscale speaks it"
why = "The protocols a node speaks, reverse engineered from Tailscale and Headscale"

[[needs]]
on = "/compat/wasmrun"
why = "The other way: Go's tsnet built for WASI, once WASMRUN has sockets"
soft = true
+++

Marko's idea: a vaxpunk system as a node on a tailnet, a WireGuard mesh
coordinated by a Headscale server, so office VMS boxes can reach each
other from anywhere. The node would be written from scratch on
vaxpunk's TCP/IP interface, no Tailscale code ported, with the protocol
details reverse engineered from Tailscale and Headscale, and Headscale
the only coordination server it needs to work with.

The other route PRD-0007 names is running Go's tsnet under WASMRUN once
it has networking. The web demo already reaches the Internet through
tailgate, a Tailscale gateway in Go built for the browser
(`web/tailgate`), outside vaxpunk.
