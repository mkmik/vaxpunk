//! The TCP/IP component, built by build.rs. No Rust code: the crate carries the build.

/// The component's image, which the root task embeds and starts.
pub const ELF: &str = concat!(env!("OUT_DIR"), "/tcpip.elf");
