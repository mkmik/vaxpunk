+++
title = "LAN drivers"
summary = "Ethernet devices of VMS's own (EWA0:, EIA0:), shared by protocols, LANCP"
aka = ["EWA0:", "EIA0:", "EWDRIVER", "LAN", "LANCP", "LANACP", "LAN Control Program", "$QIO on a LAN device", "Ethernet", "LAN failover", "LLA"]
kind = "core"
intent = "aspiration"
research = "deferred"
status = "todo"
refs = ["docs/prd/0002-networking.md", "https://docs.vmssoftware.com/vsi-openvms-io-user-s-reference-manual/"]

[[needs]]
on = "//exec/qio"
why = "Protocols and programs open a LAN device with $QIO"

[[needs]]
on = "//drivers/interrupt-io"
why = "Frames arrive by interrupt"

[[needs]]
on = "//platform/virtio"
why = "virtio-net would be the device"

[[needs]]
fuzzy = "A native TCP/IP stack in the executive, above the LAN driver, in lwIP's place"
why = "Nothing else on vaxpunk would use the LAN device"
+++

On VMS each network adapter is a LAN device, `EWA0:` or `EIA0:` and so
on, driven by a LAN driver that several protocols share at once: TCP/IP,
DECnet, LAT, cluster traffic (SCS) and programs that open the device
with `$QIO` and a protocol type or SAP of their own. `LANCP` and its
server, `LANACP`, set adapters' speed and duplex, keep the permanent
device database, make failover sets (`LLA0:`) and VLANs, and serve MOP
downline loads.

vaxpunk has no LAN device: the virtio-net driver is inside the TCP/IP
component, in C, below the port, with lwIP
([PRD-0002](../../docs/prd/0002-networking.md)). PRD-0002's replace-later
path is a native stack and driver in MACRO-32 or BLISS under the same
port; a LAN device that the executive owns would be the VMS way to do
that, and would let LAT or other protocols share the adapter.
