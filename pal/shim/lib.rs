//! The shim, built by build.rs. No Rust code: the crate carries the build.

/// The Limine executable that loads seL4 and the root task.
pub const ELF: &str = concat!(env!("OUT_DIR"), "/shim.elf");
