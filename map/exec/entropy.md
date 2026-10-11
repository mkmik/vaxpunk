+++
title = "$GET_ENTROPY"
summary = "VSI's service for random bytes good enough for keys, from virtio-rng"
aka = ["$GET_ENTROPY", "SYS$GET_ENTROPY", "GETENTROPY", "PAL call 0x49", "virtio-rng", "random bytes", "SS$_RETRY"]
kind = "core"
intent = "goal"
research = "done"
refs = ["docs/adr/0031-entropy-from-virtio-rng.md", "docs/design/0001-pal-interface.md", "vms/exec/entropy.mar", "pal/src/main.c", "vms/sysexe/svctest.mar", "scripts/run-qemu.sh"]

[[feature]]
id = "service"
title = "$GET_ENTROPY buffer, buflen: up to 256 bytes; SS$_BADPARAM, SS$_ACCVIO, SS$_NOSUCHDEV"
status = "done"
note = "ADR-0031 decision 1; entropy.mar, SVCTEST checks it"

[[feature]]
id = "pal-call"
title = "The PAL's GETENTROPY, call 0x49, reads virtio-rng until the buffer is full"
status = "done"
needs = ["//platform/virtio"]
note = "ADR-0031 decision 2; run-qemu.sh and the web demo add the device"

[[feature]]
id = "marked"
title = "Marked in the sources and the API reference as a departure from VAX and Alpha VMS"
status = "done"
note = "ADR-0031 decision 5: the vaxpunk: paragraph, which //host/apidoc tags Not VMS"

[[feature]]
id = "real-hardware"
title = "A source on real hardware: RNDR, or a generator in the executive seeded from a device"
status = "todo"
note = "ADR-0031 alternatives: worth it when a machine without a fast device matters"

[[needs]]
on = "//platform/pal"
why = "GETENTROPY is a PAL call"

[[needs]]
on = "//exec/system-services"
why = "In the vector's Security group"
+++

VAX and Alpha VMS had no source of random bytes; programs seeded their
generators from the time and the counters `$GETJPI`, `$GETRMI` and
`$GETDVI` return. VSI added `SYS$GET_ENTROPY`, after FreeBSD's
`getentropy()`: at most 256 bytes a call, `SS$_NORMAL` or `SS$_RETRY`.
OpenSSL on VSI OpenVMS uses it where it exists.

vaxpunk has it with VSI's name, arguments and limit
([ADR-0031](../../docs/adr/0031-entropy-from-virtio-rng.md)), because
on QEMU the old seed can be guessed. `vms/exec/entropy.mar` checks the
buffer and calls the PAL's `GETENTROPY`, which reads a virtio-rng
device QEMU fills from the host's generator; there is no pool and no
fallback, and without the device the service fails with
`SS$_NOSUCHDEV`. It is for keys: SSL3, WASMRUN's `random_get`, SSH and
a tailnet node. Salts keep coming from `$GETTIM`.
