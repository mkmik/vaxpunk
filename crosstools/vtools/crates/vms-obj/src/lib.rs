//! VMS-style object modules, object libraries and executable images for ARM64,
//! transliterated from the OpenVMS Alpha formats. Works on byte buffers only, so
//! the OS image loader can reuse it.
//!
//! The formats are defined in `docs/object-format.md`, `docs/library-format.md`
//! and `docs/image-format.md`.

#![no_std]

extern crate alloc;

mod record;

pub mod exe;
pub mod obj;
pub mod olb;
pub mod reloc;

pub use record::Error;

/// Architecture code for ARM64, in `EMH$L_ARCH1` (objects) and `EIHD$L_ARCH`
/// (images). The same number ELF uses for AArch64 (`EM_AARCH64`).
pub const ARCH_ARM64: u32 = 183;
