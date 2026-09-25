# shim

The shim is the Limine-protocol executable. Limine loads it with two modules,
`kernel` (seL4) and `roottask`. The shim places both in physical memory and
enters seL4 in the same state the stock seL4 elfloader leaves it in.

## seL4 entry contract

This contract was read from source, not docs: seL4 16.0.0
(`src/arch/arm/64/head.S`, `init_kernel()` in `src/arch/arm/kernel/boot.c`)
and the elfloader in seL4_tools `7dd5ba1`, the commit the 16.0.0 sel4test
manifest pins (`elfloader-tool/src/common.c`, `src/arch-arm/sys_boot.c`,
`src/arch-arm/64/mmu.c`, `src/arch-arm/armv/armv8-a/64/mmu.S`). Check it
again when bumping the seL4 tag.

### Registers

| Register | Value |
| --- | --- |
| `x0` | user image physical start, page aligned |
| `x1` | user image physical end: start + (max vaddr rounded up to a page − min vaddr) |
| `x2` | physical − virtual offset of the user image; seL4 computes vaddr = paddr − `x2` |
| `x3` | user image entry point (virtual) |
| `x4` | DTB physical address, 0 if none |
| `x5` | DTB size (FDT `totalsize`), 0 if none |

### Memory placement

- Kernel ELF segments go to their physical addresses (`p_paddr`), with BSS zeroed.
- The DTB is copied to the first page boundary after the kernel's physical end.
- The user image starts at the first page boundary after the DTB. Each segment
  goes to start + (`p_vaddr` − min vaddr). Min vaddr must be page aligned.

### CPU and MMU state

- EL1, `DAIF` masked, MMU, D-cache and I-cache on (`SCTLR_EL1.{M,C,I}`).
- `MAIR_EL1 = 0x0000aaff440c0400`, which gives Attr0 Device-nGnRnE,
  Attr1 Device-nGnRE, Attr2 Device-GRE, Attr3 Normal NC, Attr4 Normal WB and
  Attr5 Normal WT. At EL1, seL4 never writes `MAIR_EL1` and indexes it by
  these slots (`enum mair_types` in `src/arch/arm/64/kernel/vspace.c`).
- `TCR_EL1`: `T0SZ = T1SZ = 16` (48-bit VA), 4 KiB granule for both halves,
  inner and outer write-back write-allocate walks, non-shareable walks
  (single-core build), 16-bit ASIDs (`AS`), and
  `IPS = ID_AA64MMFR0_EL1.PARange`. At EL1, seL4 never writes `TCR_EL1`.
- `TTBR1_EL1`: maps the kernel at its virtual base with 2 MiB Normal (Attr4)
  blocks, from the kernel's first vaddr to the end of that 1 GiB.
- `TTBR0_EL1`: 2 MiB Normal identity blocks covering the code that jumps to
  the kernel. seL4 replaces both TTBRs in `activate_kernel_vspace()`.
- `VBAR_EL1`: the loader's own table. seL4 installs its own vectors.
- Caches: D-cache cleaned before the MMU goes off, I-cache invalidated.

Not reproduced: after the user image, the elfloader writes the user image's
ELF program headers into the next page (`keep_headers`) for sel4runtime.
Nothing in seL4 reads that page, and the vaxpunk root task does not use it.
