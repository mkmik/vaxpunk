//! The system disk, built by build.rs. No Rust code: the crate carries the build.

/// The system disk, a Files-11 ODS-2 volume: EXEC.EXE, the MACRO-32
/// executive the PAL starts, and the images of its processes in
/// [SYSEXE], SYSTEM's files in [SYSMGR]. QEMU attaches it as a read-only
/// virtio disk.
pub const DISK: &str = concat!(env!("OUT_DIR"), "/sysdisk.img");
