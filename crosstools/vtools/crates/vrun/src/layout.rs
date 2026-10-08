// Guest physical layout shared by vrun and the boot stub. build.rs passes
// these to the assembler with --defsym, so they are defined once, here.

/// PL011 UART on QEMU virt.
pub const UART: u64 = 0x0900_0000;
/// Start of RAM on QEMU virt.
pub const RAM_BASE: u64 = 0x4000_0000;
/// Where vrun loads everything. Without -kernel, QEMU keeps its device tree
/// in the first megabyte of RAM.
pub const LOAD_BASE: u64 = 0x4020_0000;
/// Room for the stub's code, at LOAD_BASE.
pub const STUB_MAX: u64 = 0x1_0000;
/// The boot block vrun fills in for the stub, followed by the stub's data.
pub const BOOT: u64 = LOAD_BASE + STUB_MAX;
/// Top of the stub's own stack, and the end of the stub region.
pub const EL1_STACK: u64 = BOOT + 0x1_0000;
