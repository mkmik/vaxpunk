//! The PAL, built by build.rs. No Rust code: the crate carries the build.

/// The root task seL4 starts, a Limine module the shim loads.
pub const ELF: &str = concat!(env!("OUT_DIR"), "/roottask.elf");
