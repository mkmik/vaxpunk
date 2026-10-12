+++
title = "C cross-compilation path"
summary = "A stock AArch64 gcc on the host, with flags that fit the calling standard"
aka = ["gcc", "aarch64-elf-gcc", "aarch64-linux-gnu-gcc", "CROSS_COMPILE", "vms/c.rs"]
kind = "tool"
intent = "induced"
research = "done"
refs = ["docs/adr/0032-c-run-time-library-and-ssl3-on-mbed-tls.md", "crosstools/vtools/docs/velf.md", "vms/c.rs"]

[[feature]]
id = "gcc"
title = "C compiled freestanding by aarch64-elf-gcc or aarch64-linux-gnu-gcc"
status = "done"
note = "velf::gcc_with; CROSS_COMPILE picks another"

[[feature]]
id = "system-disk"
title = "The C run-time library, SSL3 and Mbed TLS built into libraries for the system disk"
status = "done"
note = "vms/c.rs, ADR-0032"

[[feature]]
id = "calling-standard"
title = "gcc taught the argument count in x9 and sign extension"
status = "todo"
note = "ADR-0032 alternatives: worth it once C calls many routines that read their count; jackets meanwhile"

[[feature]]
id = "fp"
title = "Floating point allowed, once exception frames keep the FP registers"
status = "todo"
note = "velf.md: -mgeneral-regs-only"

+++

How C gets into vaxpunk: there is no C compiler on the system, so a stock
cross gcc (`aarch64-elf-gcc` on the Mac, `aarch64-linux-gnu-gcc` on
Linux, or `CROSS_COMPILE`'s) compiles C on the host with `velf::GCC_FLAGS`, which keep x18,
frame pointers and the FP registers as the calling standard and the PAL
need, and `velf` converts the objects. `vms/c.rs` builds the C run-time
library, SSL3 and Mbed TLS this way, downloading Mbed TLS's release into
the user's cache.

It exists only because the C run-time library and SSL3 needed C
([ADR-0032](../../adr/0032-c-run-time-library-and-ssl3-on-mbed-tls.md)).
gcc sets no argument count and its pointers are 64-bit, so system
services are called through jackets. A native C compiler is `t8y:/lang/c`.
