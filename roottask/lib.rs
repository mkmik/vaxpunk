//! The root task, built by build.rs. No Rust code: the crate carries the build.

/// The root task seL4 starts, a Limine module the shim loads.
pub const ELF: &str = concat!(env!("OUT_DIR"), "/roottask.elf");

/// The MACRO-32 image the root task loads and calls, a Limine module the shim
/// appends to the root task.
pub const EXE: &str = concat!(env!("OUT_DIR"), "/exec.exe");
