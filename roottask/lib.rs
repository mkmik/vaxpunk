//! The root task, built by build.rs. No Rust code: the crate carries the build.

/// The root task seL4 starts, a Limine module the shim loads.
pub const ELF: &str = concat!(env!("OUT_DIR"), "/roottask.elf");

/// The boot volume: EXEC.EXE, the MACRO-32 executive the root task starts,
/// and the images of its processes. A Limine module the shim appends to the
/// root task.
pub const VOL: &str = concat!(env!("OUT_DIR"), "/sys.vol");
