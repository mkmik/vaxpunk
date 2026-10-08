//! seL4, built by build.rs. No Rust code: the crate carries the build.

/// The kernel image, a Limine module the shim loads.
pub const ELF: &str = concat!(env!("OUT_DIR"), "/install/bin/kernel.elf");
